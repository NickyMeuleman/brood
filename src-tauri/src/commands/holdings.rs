use crate::db::types::InstrumentType;
use crate::db::Db;
use crate::AppError;
use chrono::{NaiveDate, Utc};
use itertools::Itertools;
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
    unit_price_basis: Decimal,
    market_value: Decimal,
    unrealised_gain: Decimal,
    pct_gain: Decimal,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct Envelope {
    holdings: Vec<EnvelopeHolding>,
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

    let mut holdings = HashMap::new();
    // listing_id -> sum_of_remaining_costs
    let mut total_costs: HashMap<i64, Decimal> = HashMap::new();

    for lot in open_lots {
        let initial = parse_decimal(&lot.qty_at_acquisition, "qty at acquisition")?;
        let sold = sales.get(&lot.id).copied().unwrap_or(Decimal::ZERO);
        let remaining = initial - sold;
        if remaining <= Decimal::ZERO {
            continue;
        }

        let listing_id = lot.listing_id;
        let unit_price = parse_decimal(&lot.price_per_unit, "lot price per unit")?;
        let cost = unit_price * remaining;
        *total_costs.entry(lot.listing_id).or_default() += cost;

        // or_insert: identity fields are identical for all lots of the same listing,
        // so taking them from the first lot encountered is correct.
        let holding = holdings.entry(listing_id).or_insert(Holding {
            listing_id,
            currency_code: lot.currency_code,
            exchange_mic: lot.exchange_mic,
            ticker: lot.ticker,
            name: lot.name,
            isin: lot.isin,
            instrument_type: lot.instrument_type,
            quantity: Decimal::ZERO,
            unit_price: Decimal::ZERO,
            unit_price_basis: Decimal::ZERO,
        });
        holding.quantity += remaining;
    }

    for (listing_id, h) in holdings.iter_mut() {
        if let Some(&price) = latest_prices.get(listing_id) {
            h.unit_price = price;
        }
        if let Some(&cost) = total_costs.get(listing_id) {
            h.unit_price_basis = cost / h.quantity;
        }
    }

    let envelope_holdings = holdings
        .into_values()
        .map(|h| {
            // always present: populated alongside holdings
            let total_cost = total_costs[&h.listing_id];
            let market_value = h.quantity * h.unit_price;
            let unrealised_gain = market_value - total_cost;
            let pct_gain = unrealised_gain / total_cost;

            EnvelopeHolding {
                unit_price_basis: h.unit_price_basis,
                listing_id: h.listing_id,
                ticker: h.ticker,
                exchange_mic: h.exchange_mic,
                instrument_type: h.instrument_type,
                quantity: h.quantity,
                currency_code: h.currency_code,
                unit_price: h.unit_price,
                isin: h.isin,
                name: h.name,
                market_value,
                unrealised_gain,
                pct_gain,
            }
        })
        .sorted_unstable_by(|a, b| a.ticker.cmp(&b.ticker))
        .collect();

    Ok(Envelope {
        holdings: envelope_holdings,
    })
}
