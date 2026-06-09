use crate::commands::lot_data::load_lot_records;
use crate::commands::{
    period_start, CurrencyPair, NetGross, Performance, Period, PeriodContext, Snapshot,
};
use crate::db::types::InstrumentType;
use crate::db::Db;
use crate::{parse_decimal, AppError};
use chrono::Utc;
use rust_decimal::Decimal;
use serde::Serialize;
use specta::Type;
use std::collections::HashMap;
use tauri::State;

#[derive(Default, Debug, Clone)]
struct CostAccumulator {
    /// local: total listing currency cost, no FX impact
    /// eur: total EUR cost that takes into account historical FX rates
    cost: CurrencyPair<Decimal>,
    /// local: Fees normalised to listing currency via historical cross-rates.
    /// For holdings where some fees were paid in a different currency (e.g. EUR fees on a USD stock),
    /// this is a derived arithmetic intermediate, not an actual payment in listing currency.
    /// eur: Fees normalised to eur via historical cross-rates.
    /// For holdings where some fees were paid in a different currency (e.g. USD fees on a EUR stock),
    /// this is a derived arithmetic intermediate, not an actual payment in listing currency.
    fees: CurrencyPair<Decimal>,
}

#[derive(Debug, Clone)]
struct Holding {
    listing_id: i64,
    isin: String,
    name: String,
    ticker: String,
    exchange_mic: String,
    instrument_type: InstrumentType,
    currency_code: String,

    quantity: Decimal,
    unit_price: Decimal,

    period_start_quantity: Decimal,
    period_start_unit_price: Option<Decimal>,

    all_time: CostAccumulator,
    period: CostAccumulator,
}

/// sent to frontend, includes all derived values
#[derive(Debug, Clone, Serialize, Type)]
struct EnvelopeHolding {
    listing_id: i64,
    isin: String,
    name: String,
    ticker: String,
    exchange_mic: String,
    instrument_type: InstrumentType,
    currency_code: String,
    current: Snapshot,
    all_time: PeriodContext,
    period: PeriodContext,
}

/// Portfolio-level EUR aggregates.
#[derive(Debug, Clone, Serialize, Type)]
pub struct TotalsEUR {
    value: Decimal,
    period_start_value: Option<Decimal>,
    all_time: Performance,
    period: Performance,
}

/// manual impl to start at Some(0) instead of None (start valid at 0 instead of invalid)
impl Default for TotalsEUR {
    fn default() -> Self {
        Self {
            value: Decimal::ZERO,
            period_start_value: Some(Decimal::ZERO),
            all_time: Performance::default(),
            period: Performance::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct Envelope {
    holdings: Vec<EnvelopeHolding>,
    totals: TotalsEUR,
}

#[derive(Debug, Clone, Copy, Default)]
struct PeriodStart {
    value: Option<CurrencyPair<Decimal>>,
    unit_price: Option<CurrencyPair<Decimal>>,
}

#[tauri::command]
#[specta::specta]
pub async fn get_holdings(db: State<'_, Db>, period: Period) -> Result<Envelope, AppError> {
    let today = Utc::now().date_naive();
    let period_start_date = period_start(today, period);
    let lot_records = load_lot_records(&db.pool).await?;

    // listing_id -> latest close
    let latest_prices: HashMap<i64, Decimal> = sqlx::query!(
        r#"
        SELECT
            listing_id,
            close
        FROM (
            SELECT 
                listing_id, 
                close, 
                ROW_NUMBER() OVER (PARTITION BY listing_id ORDER BY date DESC) as rn
            FROM price_history
            WHERE date <= ?1
        )
        WHERE rn = 1
        "#,
        today
    )
    .fetch_all(&db.pool)
    .await?
    .into_iter()
    .map(|r| {
        let price = parse_decimal(&r.close, "price_history close")?;
        Ok((r.listing_id, price))
    })
    .collect::<Result<_, AppError>>()?;

    let latest_rates: HashMap<String, Decimal> = sqlx::query!(
        r#"
        SELECT
            currency,
            rate_to_eur
        FROM (
            SELECT
                currency,
                rate_to_eur,
                ROW_NUMBER() OVER (PARTITION BY currency ORDER BY date DESC) as rn
            FROM fx_rate
            WHERE date <= ?1
        )
        WHERE rn = 1
        "#,
        today
    )
    .fetch_all(&db.pool)
    .await?
    .into_iter()
    .map(|r| {
        let rate = parse_decimal(&r.rate_to_eur, "fx_rate rate_to_eur")?;
        Ok((r.currency, rate))
    })
    .collect::<Result<_, AppError>>()?;

    let period_start_rates: HashMap<String, Decimal> = if let Some(start) = period_start_date {
        sqlx::query!(
            r#"
        SELECT
            currency,
            rate_to_eur
        FROM (
            SELECT
                currency,
                rate_to_eur,
                ROW_NUMBER() OVER (PARTITION BY currency ORDER BY date DESC) as rn
            FROM fx_rate
            WHERE date <= ?1
        )
        WHERE rn = 1
        "#,
            start
        )
        .fetch_all(&db.pool)
        .await?
        .into_iter()
        .map(|r| {
            let rate = parse_decimal(&r.rate_to_eur, "fx_rate rate_to_eur")?;
            Ok((r.currency, rate))
        })
        .collect::<Result<_, AppError>>()?
    } else {
        HashMap::new()
    };

    // splits between period_start and now, to be applied to affected holdings
    let splits = if let Some(start) = period_start_date {
        sqlx::query!(
            r#"
    SELECT
        l.id AS listing_id,
        ca.ratio_from,
        ca.ratio_to
    FROM corporate_action ca
    JOIN listing l ON l.instrument_id = ca.instrument_id
    WHERE action_type IN ('SPLIT', 'REVERSE_SPLIT')
      AND effective_date > ?1 
      AND effective_date <= ?2
    "#,
            start,
            today
        )
        .fetch_all(&db.pool)
        .await?
    } else {
        Vec::new()
    };

    let mut split_multipliers: HashMap<i64, Decimal> = HashMap::new();
    for split in splits {
        let from = parse_decimal(&split.ratio_from, "ratio_from")?;
        let to = parse_decimal(&split.ratio_to, "ratio_to")?;
        let factor = from / to;

        if let Some(listing_id) = split.listing_id {
            *split_multipliers.entry(listing_id).or_insert(Decimal::ONE) *= factor;
        }
    }

    let period_start_prices: HashMap<i64, Decimal> = if let Some(start) = period_start_date {
        sqlx::query!(
            r#"
        SELECT
            listing_id,
            close
        FROM (
            SELECT 
                listing_id, 
                close, 
                ROW_NUMBER() OVER (PARTITION BY listing_id ORDER BY date DESC) as rn
            FROM price_history
            WHERE date <= ?1
        )
        WHERE rn = 1
        "#,
            start
        )
        .fetch_all(&db.pool)
        .await?
        .into_iter()
        .map(|r| {
            let price = parse_decimal(&r.close, "period start price")?;
            let multiplier = split_multipliers
                .get(&r.listing_id)
                .unwrap_or(&Decimal::ONE);
            Ok((r.listing_id, price * multiplier))
        })
        .collect::<Result<_, AppError>>()?
    } else {
        HashMap::new()
    };

    let mut holdings = HashMap::new();
    for lot in lot_records.iter().filter(|l| l.is_active_at(today)) {
        let qty = lot.qty_remaining_at(today);
        if qty <= Decimal::ZERO {
            continue;
        }

        let h = holdings.entry(lot.listing_id).or_insert_with(|| Holding {
            // identity fields are identical for all lots of the same listing,
            // so taking them from the first lot encountered is correct.
            listing_id: lot.listing_id,
            isin: lot.isin.clone(),
            name: lot.name.clone(),
            ticker: lot.ticker.clone(),
            exchange_mic: lot.exchange_mic.clone(),
            instrument_type: lot.instrument_type.clone(),
            currency_code: lot.currency_code.clone(),

            quantity: Decimal::ZERO,
            unit_price: Decimal::ZERO,

            period_start_quantity: Decimal::ZERO,
            period_start_unit_price: None,

            all_time: CostAccumulator::default(),
            period: CostAccumulator::default(),
        });

        // A lot is "during the period" if it was acquired on or after period_start.
        // For CA lots this correctly uses the original trade date (propagated
        // through the parent chain), not the CA effective date.
        let acq_during_period = period_start_date
            .map(|start| lot.acquisition_date >= start)
            // all lots are "new" for "AllTime"
            .unwrap_or(true);
        // price_per_unit is always correct, even for CA-originated lots because it is recalculated
        // at CA time (old lot is closed, new lot with adjusted price is added)
        let cost_local = lot.price_per_unit * qty;
        let cost_eur = lot.cost_contribution_eur(qty);
        let fees_local = lot.fees_local_for_qty(qty);
        let fees_eur = lot.fees_eur_for_qty(qty);

        if acq_during_period {
            h.period.cost.local += cost_local;
            h.period.cost.eur += cost_eur;
            h.period.fees.local += fees_local;
            h.period.fees.eur += fees_eur;
        } else {
            h.period_start_quantity += qty;
        }

        h.quantity += qty;
        h.all_time.cost.local += cost_local;
        h.all_time.cost.eur += cost_eur;
        h.all_time.fees.local += fees_local;
        h.all_time.fees.eur += fees_eur;
    }

    for (listing_id, h) in holdings.iter_mut() {
        h.unit_price = latest_prices.get(listing_id).copied().ok_or_else(|| {
            AppError::Database(format!("Missing current price for listing_id {listing_id}"))
        })?;
        h.period_start_unit_price = period_start_prices.get(listing_id).copied();
    }

    let mut envelope_holdings: Vec<EnvelopeHolding> = holdings
        .into_values()
        .map(|h| {
            let rate = if h.currency_code == "EUR" {
                Ok(Decimal::ONE)
            } else {
                latest_rates.get(&h.currency_code).copied().ok_or_else(|| {
                    AppError::Database(format!("No FX rate for {}", h.currency_code))
                })
            }?;

            let value = h.quantity * h.unit_price;
            let value_eur = value * rate;

            // --- 1. ALL-TIME METRICS ---
            let all_time_perf = CurrencyPair {
                local: Performance::calculate(
                    Some(Decimal::ZERO),
                    value,
                    h.all_time.cost.local,
                    h.all_time.fees.local,
                ),
                // INFO: gain_eur is not the same as gain * rate
                // Cost side uses historical acquisition rates; value side uses today's rate.
                // The difference captures both price appreciation and FX movement since purchase.
                eur: Performance::calculate(
                    Some(Decimal::ZERO),
                    value_eur,
                    h.all_time.cost.eur,
                    h.all_time.fees.eur,
                ),
            };

            // --- 2. PERIOD START VALUES ---
            let period_start = match (
                period_start_date,
                h.period_start_quantity.is_zero(),
                h.period_start_unit_price,
            ) {
                // all time
                (None, _, _) => PeriodStart {
                    value: Some(CurrencyPair::default()),
                    unit_price: None,
                },
                // period starting at 0 shares, price is irrelevant, value is Some(0)
                (Some(_), true, _) => PeriodStart {
                    value: Some(CurrencyPair::default()),
                    unit_price: None,
                },
                // period with starting shares and price
                (Some(_), false, Some(start_price)) => {
                    let period_start_rate = if h.currency_code == "EUR" {
                        Decimal::ONE
                    } else {
                        period_start_rates
                            .get(&h.currency_code)
                            .copied()
                            .ok_or_else(|| {
                                AppError::Database(format!(
                                    "No period start FX rate for {} - price exists but FX missing",
                                    h.currency_code
                                ))
                            })?
                    };
                    PeriodStart {
                        value: Some(CurrencyPair {
                            local: h.period_start_quantity * start_price,
                            eur: h.period_start_quantity * start_price * period_start_rate,
                        }),
                        unit_price: Some(CurrencyPair {
                            local: start_price,
                            eur: start_price * period_start_rate,
                        }),
                    }
                }
                // period with starting shares but missing price
                (Some(_), false, None) => PeriodStart::default(),
            };

            // --- 3. PERIOD METRICS ---
            let period_perf = CurrencyPair {
                local: Performance::calculate(
                    period_start.value.map(|pair| pair.local),
                    value,
                    h.period.cost.local,
                    h.period.fees.local,
                ),
                eur: Performance::calculate(
                    period_start.value.map(|pair| pair.eur),
                    value_eur,
                    h.period.cost.eur,
                    h.period.fees.eur,
                ),
            };

            // --- 4. MAP TO ENVELOPE ---
            let get_basis = |cost: Decimal| -> Decimal {
                if h.quantity.is_zero() {
                    Decimal::ZERO
                } else {
                    cost / h.quantity
                }
            };
            Ok(EnvelopeHolding {
                // Identity
                listing_id: h.listing_id,
                isin: h.isin,
                name: h.name,
                ticker: h.ticker,
                exchange_mic: h.exchange_mic,
                instrument_type: h.instrument_type,
                currency_code: h.currency_code,

                // State
                current: Snapshot {
                    quantity: h.quantity,
                    unit_price: CurrencyPair {
                        local: h.unit_price,
                        eur: h.unit_price * rate,
                    },
                    value: CurrencyPair {
                        local: value,
                        eur: value_eur,
                    },
                    unit_price_basis: CurrencyPair {
                        local: NetGross {
                            net: get_basis(all_time_perf.local.net.cost),
                            gross: get_basis(all_time_perf.local.gross.cost),
                        },
                        eur: NetGross {
                            net: get_basis(all_time_perf.eur.net.cost),
                            // INFO: unit_price_basis_eur is not the same as unit_price_basis * rate
                            // historical FX rates are used instead of the latest one
                            gross: get_basis(all_time_perf.eur.gross.cost),
                        },
                    },
                },
                // Period metrics
                all_time: PeriodContext {
                    perf: all_time_perf,
                    ..Default::default()
                },
                period: PeriodContext {
                    start_quantity: h.period_start_quantity,
                    start_unit_price: period_start.unit_price,
                    start_value: period_start.value,
                    perf: period_perf,
                },
            })
        })
        .collect::<Result<_, AppError>>()?;
    envelope_holdings.sort_unstable_by(|a, b| a.ticker.cmp(&b.ticker));

    let mut totals = TotalsEUR::default();
    for h in &envelope_holdings {
        totals.value += h.current.value.eur;
        totals.period_start_value = totals
            .period_start_value
            .zip(h.period.start_value)
            .map(|(a, b)| a + b.eur);
        totals.all_time += h.all_time.perf.eur;
        totals.period += h.period.perf.eur;
    }
    totals.all_time.recalc_pcts(Some(Decimal::ZERO));
    totals.period.recalc_pcts(totals.period_start_value);

    Ok(Envelope {
        holdings: envelope_holdings,
        totals,
    })
}
