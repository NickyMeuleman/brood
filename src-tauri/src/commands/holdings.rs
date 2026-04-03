use crate::db::types::InstrumentType;
use crate::db::Db;
use crate::AppError;
use chrono::{Datelike, Days, Duration, Months, NaiveDate, NaiveDateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;
use std::str::FromStr;
use tauri::State;

fn parse_decimal(s: &str, ctx: &str) -> Result<Decimal, AppError> {
    Decimal::from_str(s).map_err(|_| AppError::Database(format!("Malformed {ctx}: {s}")))
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub enum Period {
    FiveDays,
    OneMonth,
    SixMonths,
    OneYear,
    FiveYears,
    Ytd,
    AllTime,
}

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
    unit_price_basis: Decimal,
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
    unit_price_eur: Decimal,
    period_start_unit_price_eur: Option<Decimal>,
    unit_price_basis: Decimal,
    unit_price_basis_eur: Decimal,
    market_value: Decimal,
    period_start_market_value: Decimal,
    market_value_eur: Decimal,
    period_start_market_value_eur: Decimal,
    gain: Decimal,
    period_gain: Decimal,
    gain_eur: Decimal,
    period_gain_eur: Decimal,
    pct_gain: Decimal,
    period_pct_gain: Decimal,
    pct_gain_eur: Decimal,
    period_pct_gain_eur: Decimal,
    cost: Decimal,
    period_cost: Decimal,
    cost_eur: Decimal,
    period_cost_eur: Decimal,
    fees_listing: Decimal,
    period_fees_listing: Decimal,
    fees_eur: Decimal,
    period_fees_eur: Decimal,
    cost_with_fees_listing: Decimal,
    period_cost_with_fees_listing: Decimal,
    cost_with_fees_eur: Decimal,
    period_cost_with_fees_eur: Decimal,
    unit_price_basis_with_fees: Decimal,
    unit_price_basis_with_fees_eur: Decimal,
    gain_with_fees: Decimal,
    period_gain_with_fees: Decimal,
    gain_with_fees_eur: Decimal,
    period_gain_with_fees_eur: Decimal,
    pct_gain_with_fees: Decimal,
    period_pct_gain_with_fees: Decimal,
    pct_gain_with_fees_eur: Decimal,
    period_pct_gain_with_fees_eur: Decimal,
    /// Fees as a fraction of acquisition cost (total_fees_eur / total_cost_eur).
    /// Unlike other fields, there is no listing-currency variant: fees and cost share
    /// the same executed_at date, so the FX rate cancels out and both formulations
    /// produce identical results.
    fee_drag: Decimal,
}

/// Portfolio-level EUR aggregates. Read directly by table footers.
#[derive(Debug, Clone, Serialize, Type, Default)]
pub struct Totals {
    market_value_eur: Decimal,
    period_start_market_value_eur: Decimal,
    gain_eur: Decimal,
    period_gain_eur: Decimal,
    gain_with_fees_eur: Decimal,
    period_gain_with_fees_eur: Decimal,
    fees_eur: Decimal,
    period_fees_eur: Decimal,
    pct_gain: Decimal,
    period_pct_gain: Decimal,
    pct_gain_with_fees: Decimal,
    period_pct_gain_with_fees: Decimal,
    cost_eur: Decimal,
    period_cost_eur: Decimal,
    cost_with_fees_eur: Decimal,
    period_cost_with_fees_eur: Decimal,
    fee_drag: Decimal,
    period_fee_drag: Decimal,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct Envelope {
    holdings: Vec<EnvelopeHolding>,
    totals: Totals,
}

async fn get_rate(
    pool: &Pool<Sqlite>,
    currency_code: &str,
    date: NaiveDate,
) -> Result<Decimal, AppError> {
    if currency_code == "EUR" {
        return Ok(Decimal::ONE);
    }

    let row = sqlx::query!(
        r#"
        SELECT rate_to_eur
        FROM fx_rate
        WHERE currency = ?1
          AND date <= ?2
        ORDER BY date DESC
        LIMIT 1
        "#,
        currency_code,
        date,
    )
    .fetch_optional(pool)
    .await
    .map_err(AppError::from)?;

    let rate = row
        .ok_or_else(|| {
            AppError::Database(format!(
                "No FX rate for {currency_code} on or before {date}"
            ))
        })?
        .rate_to_eur;

    parse_decimal(&rate, "FX rate")
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

    // trade_id -> (trade_qty, fees_in_listing_currency, fees_in_eur)
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
            unit_price_basis: Decimal::ZERO,
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
        h.unit_price_basis = h.cost / h.quantity;
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
            let gain = market_value - h.cost;
            // INFO: unrealised_gain_eur is not the same as unrealised_gain * rate
            // Cost side uses historical acquisition rates; market side uses today's rate.
            // The difference captures both price appreciation and FX movement since purchase.
            let gain_eur = market_value_eur - h.cost_eur;
            // INFO: unit_price_basis_eur is not the same as unit_price_basis * rate
            // historical FX rates are used instead of the latest one
            let unit_price_basis_eur = h.cost_eur / h.quantity;
            let pct_gain = gain / h.cost;
            // INFO: pct_gain_eur is not the same as pct_gain
            // historical FX rates are used instead of the latest one
            let pct_gain_eur = gain_eur / h.cost_eur;
            let cost_with_fees_listing = h.cost + h.fees_listing;
            let cost_with_fees_eur = h.cost_eur + h.fees_eur;
            let gain_with_fees = market_value - cost_with_fees_listing;
            let gain_with_fees_eur = market_value_eur - cost_with_fees_eur;
            let unit_price_basis_with_fees = cost_with_fees_listing / h.quantity;
            let unit_price_basis_with_fees_eur = cost_with_fees_eur / h.quantity;
            let pct_gain_with_fees = gain_with_fees / cost_with_fees_listing;
            let pct_gain_with_fees_eur = gain_with_fees_eur / cost_with_fees_eur;
            let fee_drag = h.fees_eur / h.cost_eur;

            let (
                period_start_unit_price_eur,
                period_start_market_value,
                period_start_market_value_eur,
                period_gain,
                period_gain_eur,
                period_gain_with_fees,
                period_gain_with_fees_eur,
                period_pct_gain,
                period_pct_gain_eur,
                period_pct_gain_with_fees,
                period_pct_gain_with_fees_eur,
                period_fees_listing,
                period_fees_eur,
                period_cost,
                period_cost_eur,
                period_cost_with_fees_listing,
                period_cost_with_fees_eur,
            ) = if period_start_date.is_some() {
                let (
                    period_start_market_value,
                    period_start_market_value_eur,
                    period_start_unit_price_eur,
                ) = if let Some(start_price) = h.period_start_unit_price {
                    let period_start_rate = if h.currency_code == "EUR" {
                        Decimal::ONE
                    } else {
                        period_start_rates
                            .get(&h.currency_code)
                            .copied()
                            .ok_or_else(|| {
                                AppError::Database(format!(
                            "No period start FX rate for {} — price exists but FX data missing",
                            h.currency_code
                        ))
                            })?
                    };
                    (
                        h.period_start_quantity * start_price,
                        h.period_start_quantity * start_price * period_start_rate,
                        Some(start_price * period_start_rate),
                    )
                } else {
                    // No price history at period start — position didn't exist yet
                    (Decimal::ZERO, Decimal::ZERO, None)
                };

                let period_gain = market_value - period_start_market_value - h.period_cost;
                let period_gain_eur =
                    market_value_eur - period_start_market_value_eur - h.period_cost_eur;
                let period_gain_with_fees = period_gain - h.period_fees_listing;
                let period_gain_with_fees_eur = period_gain_eur - h.period_fees_eur;

                let period_base = period_start_market_value + h.period_cost;
                let period_pct_gain = if period_base.is_zero() {
                    Decimal::ZERO
                } else {
                    period_gain / period_base
                };
                let period_base_eur = period_start_market_value_eur + h.period_cost_eur;
                let period_pct_gain_eur = if period_start_market_value_eur.is_zero() {
                    Decimal::ZERO
                } else {
                    period_gain_eur / period_base_eur
                };
                let period_pct_gain_with_fees = if period_start_market_value.is_zero() {
                    Decimal::ZERO
                } else {
                    period_gain_with_fees / period_start_market_value
                };
                let period_pct_gain_with_fees_eur = if period_start_market_value_eur.is_zero() {
                    Decimal::ZERO
                } else {
                    period_gain_with_fees_eur / period_start_market_value_eur
                };

                let period_cost_with_fees_listing = h.period_cost + h.period_fees_listing;
                let period_cost_with_fees_eur = h.period_cost_eur + h.period_fees_eur;

                (
                    period_start_unit_price_eur,
                    period_start_market_value,
                    period_start_market_value_eur,
                    period_gain,
                    period_gain_eur,
                    period_gain_with_fees,
                    period_gain_with_fees_eur,
                    period_pct_gain,
                    period_pct_gain_eur,
                    period_pct_gain_with_fees,
                    period_pct_gain_with_fees_eur,
                    h.period_fees_listing,
                    h.period_fees_eur,
                    h.period_cost,
                    h.period_cost_eur,
                    period_cost_with_fees_listing,
                    period_cost_with_fees_eur,
                )
            } else {
                // AllTime — period fields are identical to all-time fields
                (
                    Some(h.unit_price_basis * rate), // use all-time basis as period start price stand-in
                    market_value,
                    market_value_eur,
                    gain,
                    gain_eur,
                    gain_with_fees,
                    gain_with_fees_eur,
                    pct_gain,
                    pct_gain_eur,
                    pct_gain_with_fees,
                    pct_gain_with_fees_eur,
                    h.fees_listing,
                    h.fees_eur,
                    h.cost,
                    h.cost_eur,
                    cost_with_fees_listing,
                    cost_with_fees_eur,
                )
            };

            Ok(EnvelopeHolding {
                listing_id: h.listing_id,
                ticker: h.ticker,
                exchange_mic: h.exchange_mic,
                instrument_type: h.instrument_type,
                currency_code: h.currency_code,

                quantity: h.quantity,
                period_start_quantity: h.period_start_quantity,
                unit_price: h.unit_price,
                period_start_unit_price: h.period_start_unit_price,
                unit_price_eur: h.unit_price * rate,
                period_start_unit_price_eur,
                unit_price_basis: h.unit_price_basis,
                unit_price_basis_eur,
                isin: h.isin,
                name: h.name,
                cost: h.cost,
                period_cost,
                cost_eur: h.cost_eur,
                period_cost_eur,
                market_value,
                period_start_market_value,
                market_value_eur,
                period_start_market_value_eur,
                gain,
                period_gain,
                gain_eur,
                period_gain_eur,
                pct_gain,
                period_pct_gain,
                pct_gain_eur,
                period_pct_gain_eur,
                fees_listing: h.fees_listing,
                period_fees_listing,
                fees_eur: h.fees_eur,
                period_fees_eur,
                cost_with_fees_listing,
                period_cost_with_fees_listing,
                cost_with_fees_eur,
                period_cost_with_fees_eur,
                gain_with_fees,
                period_gain_with_fees,
                gain_with_fees_eur,
                period_gain_with_fees_eur,
                unit_price_basis_with_fees,
                unit_price_basis_with_fees_eur,
                pct_gain_with_fees,
                period_pct_gain_with_fees,
                pct_gain_with_fees_eur,
                period_pct_gain_with_fees_eur,
                fee_drag,
            })
        })
        .collect::<Result<_, AppError>>()?;
    envelope_holdings.sort_unstable_by(|a, b| a.ticker.cmp(&b.ticker));

    let mut totals = Totals::default();
    for h in &envelope_holdings {
        totals.market_value_eur += h.market_value_eur;
        totals.gain_eur += h.gain_eur;
        totals.cost_eur += h.cost_eur;
        totals.fees_eur += h.fees_eur;
        totals.cost_with_fees_eur += h.cost_with_fees_eur;
        totals.gain_with_fees_eur += h.gain_with_fees_eur;
        totals.period_start_market_value_eur += h.period_start_market_value_eur;
        totals.period_gain_eur += h.period_gain_eur;
        totals.period_gain_with_fees_eur += h.period_gain_with_fees_eur;
        totals.period_fees_eur += h.period_fees_eur;
        totals.period_cost_eur += h.period_cost_eur;
        totals.period_cost_with_fees_eur += h.period_cost_with_fees_eur;
    }
    totals.pct_gain = if totals.cost_eur.is_zero() {
        Decimal::ZERO
    } else {
        totals.gain_eur / totals.cost_eur
    };
    totals.pct_gain_with_fees = if totals.cost_with_fees_eur.is_zero() {
        Decimal::ZERO
    } else {
        totals.gain_with_fees_eur / totals.cost_with_fees_eur
    };
    totals.fee_drag = totals.fees_eur / totals.cost_eur;

    let period_base = totals.period_start_market_value_eur + totals.period_cost_eur;
    totals.period_pct_gain = if period_base.is_zero() {
        Decimal::ZERO
    } else {
        totals.period_gain_eur / period_base
    };
    let period_base_with_fees =
        totals.period_start_market_value_eur + totals.period_cost_with_fees_eur;
    totals.period_pct_gain_with_fees = if period_base_with_fees.is_zero() {
        Decimal::ZERO
    } else {
        totals.period_gain_with_fees_eur / period_base_with_fees
    };
    totals.period_fee_drag = if totals.period_cost_eur.is_zero() {
        Decimal::ZERO
    } else {
        totals.period_fees_eur / totals.period_cost_eur
    };

    Ok(Envelope {
        holdings: envelope_holdings,
        totals,
    })
}
