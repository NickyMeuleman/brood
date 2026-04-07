use crate::commands::{get_rate, CurrencyPair, NetGross, Period, PeriodContext};
use crate::db::types::InstrumentType;
use crate::db::Db;
use crate::{parse_decimal, AppError};
use chrono::{Datelike, Days, Months, NaiveDate, NaiveDateTime, Utc};
use rust_decimal::Decimal;
use serde::Serialize;
use specta::Type;
use std::collections::HashMap;
use tauri::State;

/// internal to backend
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
    period_start_quantity: Decimal,
    unit_price: Decimal,
    period_start_unit_price: Option<Decimal>,
    cost: Decimal,
    period_cost: Decimal,
    /// total EUR cost that takes into account historical FX rates
    cost_eur: Decimal,
    period_cost_eur: Decimal,
    /// Fees normalised to listing currency via historical cross-rates.
    /// For holdings where some fees were paid in a different currency (e.g. EUR fees on a USD stock),
    /// this is a derived arithmetic intermediate, not an actual payment in listing currency.
    fees_listing: Decimal,
    period_fees_listing: Decimal,
    /// Fees normalised to eur via historical cross-rates.
    /// For holdings where some fees were paid in a different currency (e.g. USD fees on a EUR stock),
    /// this is a derived arithmetic intermediate, not an actual payment in listing currency.
    fees_eur: Decimal,
    period_fees_eur: Decimal,
}

/// sent to frontend, includes all derived values
#[derive(Debug, Clone, Serialize, Type)]
struct EnvelopeHolding {
    // Identity
    listing_id: i64,
    isin: String,
    name: String,
    ticker: String,
    exchange_mic: String,
    instrument_type: InstrumentType,
    currency_code: String,

    // state
    quantity: Decimal,
    unit_price: Decimal,
    unit_price_eur: Decimal,
    unit_price_basis: Decimal,
    unit_price_basis_eur: Decimal,
    unit_price_basis_with_fees: Decimal,
    unit_price_basis_with_fees_eur: Decimal,
    market_value: Decimal,
    market_value_eur: Decimal,

    // period metrics
    all_time: PeriodContext,
    period: PeriodContext,
    // for fee_drag percentage: prefer the eur variants.
    // Fees as a fraction of acquisition cost (total_fees_eur / total_cost_eur).
    // fees and cost share the same executed_at date,
    // so the FX rate cancels out and both formulations produce identical results.
}

/// Portfolio-level EUR aggregates.
#[derive(Debug, Clone, Serialize, Type, Default)]
pub struct TotalsEUR {
    market_value: Decimal,
    period_start_market_value: Decimal,
    all_time: NetGross,
    period: NetGross,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct Envelope {
    holdings: Vec<EnvelopeHolding>,
    totals: TotalsEUR,
}

// Both converted at the trade's execution date. At the moment the fee was actually paid.
// fees_listing: all fees in listing currency (if needed, EUR parts are converted via historical cross-rate.)
//   Used for cost basis arithmetic in listing currency.
//   This means this is not a real payment, it's a derived intermediate
//   (parts of the total fee amount can be paid in EUR, eg. re=bel broker fees in EUR instead of listing_currency)
// fees_eur: all fees in EUR (if needed, different currency parts were converted via historical rate.)
//   Historically accurate total fees in EUR.
//   This means this is not a real payment, it's a derived intermediate
//   (parts of the total fee amount can be paid in listing_currency, not EUR)
struct TradeFeeAgg {
    trade_qty: Decimal,
    fees_listing: Decimal,
    fees_eur: Decimal,
}

struct PeriodStart {
    market_value: Decimal,
    market_value_eur: Decimal,
    unit_price_eur: Option<Decimal>,
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
            fees_listing: Decimal::ZERO,
            fees_eur: Decimal::ZERO,
        });
        entry.fees_listing += fee_listing;
        entry.fees_eur += fee_eur;
    }

    // lot_id -> original_acquisition_date
    let mut acquisition_dates: HashMap<i64, NaiveDate> = HashMap::new();
    let mut lot_fees_listing: HashMap<i64, Decimal> = HashMap::new();
    let mut lot_fees_eur: HashMap<i64, Decimal> = HashMap::new();
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
                        (agg.fees_listing * ratio, agg.fees_eur * ratio)
                    })
                    .unwrap_or_default();

                (date, fees_listing, fees_eur)
            }
            (None, Some(parent_id)) => {
                // CA lot: inherit the date from the parent, which was already processed
                // (topological order guarantees parent comes first)
                let date = acquisition_dates[&parent_id];
                let ratio = lot_costs[&r.id] / lot_costs[&parent_id];
                let fees_listing = lot_fees_listing[&parent_id] * ratio;
                let fees_eur = lot_fees_eur[&parent_id] * ratio;

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
        lot_fees_listing.insert(r.id, fees_listing);
        lot_fees_eur.insert(r.id, fees_eur);
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
            currency_code: lot.currency_code.clone(),
            exchange_mic: lot.exchange_mic,
            ticker: lot.ticker,
            name: lot.name,
            isin: lot.isin,
            instrument_type: lot.instrument_type,
            quantity: Decimal::ZERO,
            unit_price: Decimal::ZERO,
            cost: Decimal::ZERO,
            cost_eur: Decimal::ZERO,
            fees_listing: Decimal::ZERO,
            fees_eur: Decimal::ZERO,
            period_start_quantity: Decimal::ZERO,
            period_start_unit_price: None,
            period_cost: Decimal::ZERO,
            period_cost_eur: Decimal::ZERO,
            period_fees_listing: Decimal::ZERO,
            period_fees_eur: Decimal::ZERO,
        });
        // price_per_unit is always correct, even for CA-originated lots because it is recalculated
        // at CA time (old lot is closed, new lot with adjusted price is added)
        let unit_price = parse_decimal(&lot.price_per_unit, "lot price per unit")?;
        let cost = unit_price * remaining;
        //  acquisition_dates holds all lots, this open lot has to be in it
        let acquisition_date = acquisition_dates[&lot.id];
        let is_during_period = period_start_date
            .map(|start| acquisition_date >= start)
            .unwrap_or(false); // AllTime: no lots are "new"
        let rate = get_rate(&db.pool, &lot.currency_code, acquisition_date).await?;
        let cost_eur = cost * rate;
        let fees_listing = lot_fees_listing[&lot.id] * remaining_ratio;
        let fees_eur = lot_fees_eur[&lot.id] * remaining_ratio;

        if is_during_period {
            // Acquired during period
            holding.period_cost += cost;
            holding.period_cost_eur += cost_eur;
            holding.period_fees_listing += fees_listing;
            holding.period_fees_eur += fees_eur;
        } else {
            // Held at period start
            holding.period_start_quantity += remaining;
        }
        holding.quantity += remaining;
        holding.cost += cost;
        holding.cost_eur += cost_eur;
        holding.fees_listing += fees_listing;
        holding.fees_eur += fees_eur;
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

            let market_value = h.quantity * h.unit_price;
            let market_value_eur = market_value * rate;

            // --- 1. ALL-TIME METRICS ---
            let all_time_local =
                NetGross::calculate(Decimal::ZERO, market_value, h.cost, h.fees_listing);
            // INFO: gain_eur is not the same as gain * rate
            // Cost side uses historical acquisition rates; market side uses today's rate.
            // The difference captures both price appreciation and FX movement since purchase.
            let all_time_eur =
                NetGross::calculate(Decimal::ZERO, market_value_eur, h.cost_eur, h.fees_eur);

            // --- 2. PERIOD START VALUES ---
            let period_start = if let (Some(start_price), Some(_)) =
                (h.period_start_unit_price, period_start_date)
            {
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
                    market_value: h.period_start_quantity * start_price,
                    market_value_eur: h.period_start_quantity * start_price * period_start_rate,
                    unit_price_eur: Some(start_price * period_start_rate),
                }
            } else {
                PeriodStart {
                    market_value: Decimal::ZERO,
                    market_value_eur: Decimal::ZERO,
                    unit_price_eur: None,
                }
            };

            // --- 3. PERIOD METRICS ---
            let period_local = if period_start_date.is_some() {
                NetGross::calculate(
                    period_start.market_value,
                    market_value,
                    h.period_cost,
                    h.period_fees_listing,
                )
            } else {
                all_time_local
            };

            let period_eur = if period_start_date.is_some() {
                NetGross::calculate(
                    period_start.market_value_eur,
                    market_value_eur,
                    h.period_cost_eur,
                    h.period_fees_eur,
                )
            } else {
                all_time_eur
            };

            // --- 4. MAP TO ENVELOPE ---
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
                quantity: h.quantity,
                unit_price: h.unit_price,
                unit_price_eur: h.unit_price * rate,
                unit_price_basis: h.cost / h.quantity,
                // INFO: unit_price_basis_eur is not the same as unit_price_basis * rate
                // historical FX rates are used instead of the latest one
                unit_price_basis_eur: h.cost_eur / h.quantity,
                unit_price_basis_with_fees: all_time_local.net.cost / h.quantity,
                unit_price_basis_with_fees_eur: all_time_eur.net.cost / h.quantity,
                market_value,
                market_value_eur,

                // Period metrics
                all_time: PeriodContext {
                    start_quantity: Decimal::ZERO,
                    start_unit_price: None,
                    start_unit_price_eur: None,
                    start_market_value: Decimal::ZERO,
                    start_market_value_eur: Decimal::ZERO,
                    metrics: CurrencyPair {
                        local: all_time_local,
                        eur: all_time_eur,
                    },
                },
                period: PeriodContext {
                    start_quantity: h.period_start_quantity,
                    start_unit_price: h.period_start_unit_price,
                    start_unit_price_eur: period_start.unit_price_eur,
                    start_market_value: period_start.market_value,
                    start_market_value_eur: period_start.market_value_eur,
                    metrics: CurrencyPair {
                        local: period_local,
                        eur: period_eur,
                    },
                },
            })
        })
        .collect::<Result<_, AppError>>()?;
    envelope_holdings.sort_unstable_by(|a, b| a.ticker.cmp(&b.ticker));

    let mut totals = TotalsEUR::default();
    for h in &envelope_holdings {
        totals.market_value += h.market_value_eur;
        totals.period_start_market_value += h.period.start_market_value_eur;

        totals.all_time += h.all_time.metrics.eur;
        totals.period += h.period.metrics.eur;
    }

    if !totals.all_time.gross.cost.is_zero() {
        totals.all_time.gross.pct_gain = totals.all_time.gross.gain / totals.all_time.gross.cost
    };
    if !totals.all_time.net.cost.is_zero() {
        totals.all_time.net.pct_gain = totals.all_time.net.gain / totals.all_time.net.cost
    };
    // same value, both pct_fees express fees as % of cost-before-fees
    totals.all_time.gross.pct_fees = totals.all_time.gross.fees / totals.all_time.gross.cost;
    totals.all_time.net.pct_fees = totals.all_time.net.fees / totals.all_time.gross.cost;

    let gross_period_base = totals.period_start_market_value + totals.period.gross.cost;
    if !gross_period_base.is_zero() {
        totals.period.gross.pct_gain = totals.period.gross.gain / gross_period_base
    };
    let net_period_base = totals.period_start_market_value + totals.period.net.cost;
    if !net_period_base.is_zero() {
        totals.period.net.pct_gain = totals.period.net.gain / net_period_base;
    };
    if !totals.period.gross.cost.is_zero() {
        totals.period.gross.pct_fees = totals.period.gross.fees / totals.period.gross.cost;
        totals.period.net.pct_fees = totals.period.gross.fees / totals.period.gross.cost;
    };

    Ok(Envelope {
        holdings: envelope_holdings,
        totals,
    })
}
