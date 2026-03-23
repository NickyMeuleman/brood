use crate::db::types::InstrumentType;
use crate::db::Db;
use crate::AppError;
use chrono::{NaiveDate, NaiveDateTime, Utc};
use rust_decimal::Decimal;
use serde::Serialize;
use specta::Type;
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;
use std::str::FromStr;
use tauri::State;

fn parse_decimal(s: &str, ctx: &str) -> Result<Decimal, AppError> {
    Decimal::from_str(s).map_err(|_| AppError::Database(format!("Malformed {ctx}: {s}")))
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
    quantity: Decimal,
    currency_code: String,
    unit_price: Decimal,
    unit_price_basis: Decimal,
    total_cost: Decimal,
    /// total EUR cost that takes into account historical FX rates
    total_cost_eur: Decimal,
    /// Fees normalised to listing currency via historical cross-rates.
    /// For holdings where some fees were paid in a different currency (e.g. EUR fees on a USD stock),
    /// this is a derived arithmetic intermediate, not an actual payment in listing currency.
    total_fees_listing: Decimal,
    /// Fees normalised to eur via historical cross-rates.
    /// For holdings where some fees were paid in a different currency (e.g. USD fees on a EUR stock),
    /// this is a derived arithmetic intermediate, not an actual payment in listing currency.
    total_fees_eur: Decimal,
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
    quantity: Decimal,
    currency_code: String,
    unit_price: Decimal,
    /// uses current FX rate
    unit_price_eur: Decimal,
    unit_price_basis: Decimal,
    /// uses historical FX rates
    unit_price_basis_eur: Decimal,
    market_value: Decimal,
    /// uses current FX rate
    market_value_eur: Decimal,
    unrealised_gain: Decimal,
    /// uses historical FX rates
    unrealised_gain_eur: Decimal,
    pct_gain: Decimal,
    /// uses historical FX rates
    pct_gain_eur: Decimal,
    total_cost: Decimal,
    /// uses historical FX rates
    total_cost_eur: Decimal,

    total_fees_listing: Decimal,
    total_fees_eur: Decimal,
    total_with_fees_listing: Decimal,
    total_with_fees_eur: Decimal,
    unit_price_basis_with_fees: Decimal,
    unit_price_basis_with_fees_eur: Decimal,
    unrealised_gain_with_fees: Decimal,
    unrealised_gain_with_fees_eur: Decimal,
    pct_gain_with_fees: Decimal,
    pct_gain_with_fees_eur: Decimal,
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
    unrealised_gain_eur: Decimal,
    unrealised_gain_with_fees_eur: Decimal,
    total_fees_eur: Decimal,
    pct_gain: Decimal,
    pct_gain_with_fees: Decimal,
    total_cost_eur: Decimal,
    total_with_fees_eur: Decimal,
    fee_drag: Decimal,
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
pub async fn get_holdings(db: State<'_, Db>) -> Result<Envelope, AppError> {
    let today = Utc::now().date_naive();

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
            total_cost: Decimal::ZERO,
            total_cost_eur: Decimal::ZERO,
            total_fees_listing: Decimal::ZERO,
            total_fees_eur: Decimal::ZERO,
        });
        // price_per_unit is always correct, even for CA-originated lots because it is recalculated
        // at CA time (old lot is closed, new lot with adjusted price is added)
        let unit_price = parse_decimal(&lot.price_per_unit, "lot price per unit")?;
        let cost = unit_price * remaining;
        holding.total_cost += cost;

        //  acquisition_dates holds all lots, this open lot has to be in it
        let acquisition_date = acquisition_dates[&lot.id];
        let rate = get_rate(&db.pool, &lot.currency_code, acquisition_date).await?;
        let cost_eur = cost * rate;
        holding.total_cost_eur += cost_eur;

        holding.quantity += remaining;

        holding.total_fees_listing += lot_fees_listing[&lot.id] * remaining_ratio;
        holding.total_fees_eur += lot_fees_eur[&lot.id] * remaining_ratio;
    }

    for (listing_id, h) in holdings.iter_mut() {
        if let Some(&price) = latest_prices.get(listing_id) {
            h.unit_price = price;
        } else {
            return Err(AppError::Internal);
        }
        h.unit_price_basis = h.total_cost / h.quantity;
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
            let unrealised_gain = market_value - h.total_cost;
            // INFO: unrealised_gain_eur is not the same as unrealised_gain * rate
            // Cost side uses historical acquisition rates; market side uses today's rate.
            // The difference captures both price appreciation and FX movement since purchase.
            let unrealised_gain_eur = market_value_eur - h.total_cost_eur;
            // INFO: unit_price_basis_eur is not the same as unit_price_basis * rate
            // historical FX rates are used instead of the latest one
            let unit_price_basis_eur = h.total_cost_eur / h.quantity;
            let pct_gain = unrealised_gain / h.total_cost;
            // INFO: pct_gain_eur is not the same as pct_gain
            // historical FX rates are used instead of the latest one
            let pct_gain_eur = unrealised_gain_eur / h.total_cost_eur;

            let total_with_fees_listing = h.total_cost + h.total_fees_listing;
            let total_with_fees_eur = h.total_cost_eur + h.total_fees_eur;
            let unrealised_gain_with_fees = market_value - total_with_fees_listing;
            let unrealised_gain_with_fees_eur = market_value_eur - total_with_fees_eur;
            let unit_price_basis_with_fees = total_with_fees_listing / h.quantity;
            let unit_price_basis_with_fees_eur = total_with_fees_eur / h.quantity;
            let pct_gain_with_fees = unrealised_gain_with_fees / total_with_fees_listing;
            let pct_gain_with_fees_eur = unrealised_gain_with_fees_eur / total_with_fees_eur;
            let fee_drag = h.total_fees_eur / h.total_cost_eur;

            Ok(EnvelopeHolding {
                listing_id: h.listing_id,
                ticker: h.ticker,
                exchange_mic: h.exchange_mic,
                instrument_type: h.instrument_type,
                quantity: h.quantity,
                currency_code: h.currency_code,
                unit_price: h.unit_price,
                unit_price_eur: h.unit_price * rate,
                unit_price_basis: h.unit_price_basis,
                unit_price_basis_eur,
                isin: h.isin,
                name: h.name,
                total_cost: h.total_cost,
                total_cost_eur: h.total_cost_eur,
                market_value,
                market_value_eur,
                unrealised_gain,
                unrealised_gain_eur,
                pct_gain,
                pct_gain_eur,
                total_fees_listing: h.total_fees_listing,
                total_fees_eur: h.total_fees_eur,
                total_with_fees_listing,
                total_with_fees_eur,
                unrealised_gain_with_fees,
                unrealised_gain_with_fees_eur,
                unit_price_basis_with_fees,
                unit_price_basis_with_fees_eur,
                pct_gain_with_fees,
                pct_gain_with_fees_eur,
                fee_drag,
            })
        })
        .collect::<Result<_, AppError>>()?;
    envelope_holdings.sort_unstable_by(|a, b| a.ticker.cmp(&b.ticker));

    let mut totals = Totals::default();
    for h in &envelope_holdings {
        totals.market_value_eur += h.market_value_eur;
        totals.unrealised_gain_eur += h.unrealised_gain_eur;
        totals.total_cost_eur += h.total_cost_eur;
        totals.total_fees_eur += h.total_fees_eur;
        totals.total_with_fees_eur += h.total_with_fees_eur;
        totals.unrealised_gain_with_fees_eur += h.unrealised_gain_with_fees_eur;
    }
    totals.pct_gain = if totals.total_cost_eur.is_zero() {
        Decimal::ZERO
    } else {
        totals.unrealised_gain_eur / totals.total_cost_eur
    };
    totals.pct_gain_with_fees = if totals.total_with_fees_eur.is_zero() {
        Decimal::ZERO
    } else {
        totals.unrealised_gain_with_fees_eur / totals.total_with_fees_eur
    };
    totals.fee_drag = totals.total_fees_eur / totals.total_cost_eur;

    Ok(Envelope {
        holdings: envelope_holdings,
        totals,
    })
}
