use crate::commands::{
    get_rate, CurrencyPair, NetGross, Performance, Period, PeriodContext, Snapshot,
};
use crate::db::types::InstrumentType;
use crate::db::Db;
use crate::{parse_decimal, AppError};
use chrono::{Datelike, Days, Months, NaiveDate, NaiveDateTime, Utc};
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

// Both converted at the trade's execution date. At the moment the fee was actually paid.
// fees.local: all fees in listing currency (if needed, EUR parts are converted via historical cross-rate.)
//   Used for cost basis arithmetic in listing currency.
//   This means this is not a real payment, it's a derived intermediate
//   (parts of the total fee amount can be paid in EUR, eg. re=bel broker fees in EUR instead of listing_currency)
// fees_eur: all fees in EUR (if needed, different currency parts were converted via historical rate.)
//   Historically accurate total fees in EUR.
//   This means this is not a real payment, it's a derived intermediate
//   (parts of the total fee amount can be paid in listing_currency, not EUR)
struct TradeFeeAgg {
    trade_qty: Decimal,
    fees: CurrencyPair<Decimal>,
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
    let period_start_date = match period {
        Period::AllTime => None,
        Period::FiveDays => today.checked_sub_days(Days::new(5)),
        Period::OneMonth => today.checked_sub_months(Months::new(1)),
        Period::SixMonths => today.checked_sub_months(Months::new(6)),
        Period::OneYear => today.checked_sub_months(Months::new(12)),
        Period::FiveYears => today.checked_sub_months(Months::new(60)),
        Period::Ytd => NaiveDate::from_ymd_opt(today.year(), 1, 1),
    };

    let open_lots = sqlx::query!(
        r#"
        SELECT 
            l.id,
            l.qty_at_acquisition,
            l.listing_id,
            l.price_per_unit,
            li.currency_code,
            li.ticker,
            li.exchange_mic,
            i.name,
            i.isin,
            i.instrument_type as "instrument_type: InstrumentType"
        FROM lot l
        JOIN listing li ON li.id = l.listing_id
        JOIN instrument i ON i.id = l.instrument_id
        WHERE l.id NOT IN (SELECT lot_id FROM lot_close)
        "#
    )
    .fetch_all(&db.pool)
    .await
    .map_err(AppError::from)?;

    // all lots, needed because some open lots (CA-originated lots) lack info about the time of the
    // price_per_unit at original acquisition time.
    // This info is needed to build a historically correct unit_price_basis_eur
    // that uses FX rates of each moment that lot's shares were bought
    // qty_at_acquisition and price_per_unit are needed for accurate fee proportional attribution
    let all_lots = sqlx::query!(
        r#"
        SELECT
            l.id,
            l.parent_lot_id,
            l.source_trade_id,
            l.qty_at_acquisition,
            l.price_per_unit,
            t.executed_at AS "executed_at: NaiveDateTime"  -- NULL for CA lots
        FROM lot l
        LEFT JOIN trade t ON t.id = l.source_trade_id
        ORDER BY l.id ASC
        "#
    )
    .fetch_all(&db.pool)
    .await
    .map_err(AppError::from)?;

    let fee_rows = sqlx::query!(
        r#"
        SELECT
            t.id             AS trade_id,
            t.quantity       AS trade_qty,
            t.executed_at    AS "executed_at: NaiveDateTime",
            li.currency_code AS listing_currency,
            tf.amount        AS fee_amount,
            tf.currency_code AS fee_currency
        FROM trade t
        JOIN trade_fee tf ON tf.trade_id = t.id
        JOIN listing li   ON li.id = t.listing_id
        "#
    )
    .fetch_all(&db.pool)
    .await
    .map_err(AppError::from)?;

    // trade_id -> TradeFeeAgg
    let mut fee_by_trade: HashMap<i64, TradeFeeAgg> = HashMap::new();
    for r in fee_rows {
        let fee = parse_decimal(&r.fee_amount, "fee amount")?;
        let date = r.executed_at.date();
        let fee_eur = fee * get_rate(&db.pool, &r.fee_currency, date).await?;
        let fee_listing = if r.fee_currency == r.listing_currency {
            // no conversion needed, avoids precision loss
            fee
        } else {
            fee_eur / get_rate(&db.pool, &r.listing_currency, date).await?
        };
        let entry = fee_by_trade.entry(r.trade_id).or_insert(TradeFeeAgg {
            // trade_qty is identical across all fee rows for the same trade
            trade_qty: parse_decimal(&r.trade_qty, "trade qty")?,
            fees: CurrencyPair::default(),
        });
        entry.fees.local += fee_listing;
        entry.fees.eur += fee_eur;
    }

    // lot_id -> original_acquisition_date
    let mut acquisition_dates: HashMap<i64, NaiveDate> = HashMap::new();
    let mut lot_fees: HashMap<i64, CurrencyPair<Decimal>> = HashMap::new();
    let mut lot_costs: HashMap<i64, Decimal> = HashMap::new(); // for CA fee proportion only

    for r in all_lots {
        let qty = parse_decimal(&r.qty_at_acquisition, "lot qty")?;
        let price = parse_decimal(&r.price_per_unit, "lot price")?;
        lot_costs.insert(r.id, qty * price);

        let (date, fees_listing, fees_eur) = match (r.source_trade_id, r.parent_lot_id) {
            (Some(trade_id), _) => {
                // Trade lot: use the trade's executed_at directly
                let date = r
                    .executed_at
                    .ok_or_else(|| {
                        AppError::Database(format!("Trade lot {} has no executed_at", r.id))
                    })?
                    .date();
                let (fees_listing, fees_eur) = fee_by_trade
                    .get(&trade_id)
                    .map(|agg| {
                        let ratio = qty / agg.trade_qty;
                        (agg.fees.local * ratio, agg.fees.eur * ratio)
                    })
                    .unwrap_or_default();

                (date, fees_listing, fees_eur)
            }
            (None, Some(parent_id)) => {
                // CA lot: inherit the date from the parent, which was already processed
                // (topological order guarantees parent comes first)
                let date = acquisition_dates[&parent_id];
                let ratio = lot_costs[&r.id] / lot_costs[&parent_id];
                let fees_listing = lot_fees[&parent_id].local * ratio;
                let fees_eur = lot_fees[&parent_id].eur * ratio;

                (date, fees_listing, fees_eur)
            }
            (None, None) => {
                return Err(AppError::Database(format!(
                    "Lot {} has neither source_trade_id nor parent_lot_id",
                    r.id
                )))
            }
        };

        acquisition_dates.insert(r.id, date);
        lot_fees.insert(
            r.id,
            CurrencyPair {
                local: fees_listing,
                eur: fees_eur,
            },
        );
    }

    let mut sales = HashMap::new();
    for r in sqlx::query!(
        r#"
        SELECT origin_lot_id, quantity
        FROM sell_allocation
        WHERE origin_lot_id NOT IN (SELECT lot_id FROM lot_close)
        "#
    )
    .fetch_all(&db.pool)
    .await
    .map_err(AppError::from)?
    {
        let qty = parse_decimal(&r.quantity, "sale quantity")?;
        *sales.entry(r.origin_lot_id).or_insert(Decimal::ZERO) += qty;
    }

    // listing_id -> latest close
    let latest_prices: HashMap<i64, Decimal> = sqlx::query!(
        r#"
        SELECT listing_id, close
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

    // seperate batch query for latest_prices instead of calling get_rate() with the current date
    // over and over for performance
    // currency_code -> rate_to_eur
    let latest_rates: HashMap<String, Decimal> = sqlx::query!(
        r#"
        SELECT currency, rate_to_eur
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
        let rate = parse_decimal(&r.rate_to_eur, "fx_rate price_to_eur")?;
        Ok((r.currency, rate))
    })
    .collect::<Result<_, AppError>>()?;

    let period_start_rates: HashMap<String, Decimal> = if let Some(start) = period_start_date {
        sqlx::query!(
            r#"
        SELECT currency, rate_to_eur
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
    SELECT l.id AS listing_id, ca.ratio_from, ca.ratio_to
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
        SELECT listing_id, close
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
    for lot in open_lots {
        let initial = parse_decimal(&lot.qty_at_acquisition, "qty at acquisition")?;
        let sold = sales.get(&lot.id).copied().unwrap_or(Decimal::ZERO);
        let remaining = initial - sold;
        if remaining <= Decimal::ZERO {
            continue;
        }
        let remaining_ratio = remaining / initial;
        let listing_id = lot.listing_id;

        // or_insert: identity fields are identical for all lots of the same listing,
        // so taking them from the first lot encountered is correct.
        let holding = holdings.entry(listing_id).or_insert(Holding {
            listing_id,
            isin: lot.isin,
            name: lot.name,
            ticker: lot.ticker,
            exchange_mic: lot.exchange_mic,
            instrument_type: lot.instrument_type,
            currency_code: lot.currency_code.clone(),

            quantity: Decimal::ZERO,
            unit_price: Decimal::ZERO,

            period_start_quantity: Decimal::ZERO,
            period_start_unit_price: None,

            all_time: CostAccumulator::default(),
            period: CostAccumulator::default(),
        });
        // price_per_unit is always correct, even for CA-originated lots because it is recalculated
        // at CA time (old lot is closed, new lot with adjusted price is added)
        let unit_price = parse_decimal(&lot.price_per_unit, "lot price per unit")?;
        let cost = unit_price * remaining;
        //  acquisition_dates holds all lots, this open lot has to be in it
        let acquisition_date = acquisition_dates[&lot.id];
        let is_during_period = period_start_date
            .map(|start| acquisition_date >= start)
            .unwrap_or(true); // AllTime: all lots are "new"
        let rate = get_rate(&db.pool, &lot.currency_code, acquisition_date).await?;
        let cost_eur = cost * rate;
        let fees_listing = lot_fees[&lot.id].local * remaining_ratio;
        let fees_eur = lot_fees[&lot.id].eur * remaining_ratio;

        if is_during_period {
            // Acquired during period
            holding.period.cost.local += cost;
            holding.period.cost.eur += cost_eur;
            holding.period.fees.local += fees_listing;
            holding.period.fees.eur += fees_eur;
        } else {
            // Held at period start
            holding.period_start_quantity += remaining;
        }
        holding.quantity += remaining;
        holding.all_time.cost.local += cost;
        holding.all_time.cost.eur += cost_eur;
        holding.all_time.fees.local += fees_listing;
        holding.all_time.fees.eur += fees_eur;
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
