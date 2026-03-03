use crate::db::types::{InstrumentType, SellAllocation};
use crate::db::Db;
use crate::AppError;
use rust_decimal::Decimal;
use serde::Serialize;
use specta::Type;
use std::collections::HashMap;
use std::str::FromStr;
use tauri::State;

#[derive(Debug, Clone, Serialize, Type)]
pub struct Holding {
    isin: String,
    name: String,
    ticker: String,
    exchange_mic: String,
    instrument_type: InstrumentType,
    currency_code: String,
    quantity: Decimal,
    avg_cost_basis: Decimal,
    current_price: Decimal,
    market_value: Decimal,
    unrealised_gain: Decimal,
}

#[tauri::command]
#[specta::specta]
pub async fn get_holdings(db: State<'_, Db>) -> Result<Vec<Holding>, AppError> {
    let lots = sqlx::query!(
        r#"
SELECT
  l.id AS "lot_id!",
  l.listing_id AS "listing_id!",
  l.qty_at_acquisition AS "qty!",
  l.price_per_unit AS "price!",
  i.isin AS "isin!",
  i.name AS "name!",
  i.instrument_type AS "instrument_type!: InstrumentType",
  l.price_currency_code AS "currency!",
  li.ticker AS "ticker!",
  li.exchange_mic AS "mic!"
FROM
  lot l
  JOIN instrument i ON l.instrument_id = i.id
  JOIN listing li ON li.instrument_id = i.id
WHERE
  (
    l.id NOT IN (
      SELECT
        lot_id
      FROM
        lot_close
    )
  );
    "#
    )
    .fetch_all(&db.pool)
    .await
    .map_err(AppError::from)?;

    let sales = sqlx::query_as!(
        SellAllocation,
        r#"
SELECT
  *
FROM
  sell_allocation sa
WHERE
  sa.origin_lot_id NOT IN (
    SELECT
      lot_id
    FROM
      lot_close
  )
    "#
    )
    .fetch_all(&db.pool)
    .await
    .map_err(AppError::from)?;

    let mut lot_sales = HashMap::new();
    for sale in sales {
        let qty = Decimal::from_str(&sale.quantity)
            .map_err(|_| AppError::Database("Malformed sell allocation data".to_string()))?;
        let qty_sold = lot_sales.entry(sale.origin_lot_id).or_insert(Decimal::ZERO);
        *qty_sold += qty;
    }

    let mut holdings = HashMap::new();
    // accumulates costs to avoid precision loss caused by calculating a running total of avg_cost_basis
    let mut total_costs = HashMap::new();

    for row in lots {
        let initial = Decimal::from_str(&row.qty)
            .map_err(|_| AppError::Database("Malformed lot qty_at_acquisition".to_string()))?;
        let sold = lot_sales.get(&row.lot_id).copied().unwrap_or(Decimal::ZERO);
        let remaining = initial - sold;

        if remaining <= Decimal::ZERO {
            continue;
        }

        let lot_price = Decimal::from_str(&row.price)
            .map_err(|_| AppError::Database("Malformed lot price_per_unit".to_string()))?;
        let lot_cost = lot_price * remaining;

        let holding = holdings.entry(row.listing_id).or_insert(Holding {
            isin: row.isin,
            name: row.name,
            ticker: row.ticker,
            exchange_mic: row.mic,
            instrument_type: row.instrument_type,
            currency_code: row.currency,
            quantity: Decimal::ZERO,
            avg_cost_basis: Decimal::ZERO,
            current_price: Decimal::ZERO,
            market_value: Decimal::ZERO,
            unrealised_gain: Decimal::ZERO,
        });

        let total_cost = total_costs.entry(row.listing_id).or_insert(Decimal::ZERO);
        *total_cost += lot_cost;

        holding.quantity += remaining;
        debug_assert!(holding.quantity > Decimal::ZERO);
        holding.avg_cost_basis = *total_cost / holding.quantity;
    }

    // PERF: future enhancement: only fetch prices in holdings instead of all prices
    let prices = sqlx::query!(
        r#"
SELECT
    ph.listing_id,
    ph.close
FROM price_history ph
WHERE ph.date = (
    SELECT MAX(ph2.date)
    FROM price_history ph2
    WHERE ph2.listing_id = ph.listing_id
)
 "#
    )
    .fetch_all(&db.pool)
    .await
    .map_err(AppError::from)?;

    let price_map: HashMap<i64, Decimal> = prices
        .into_iter()
        .map(|r| {
            let price = Decimal::from_str(&r.close)
                .map_err(|_| AppError::Database("Malformed price_history".to_string()))?;
            Ok((r.listing_id, price))
        })
        .collect::<Result<_, AppError>>()?;

    for (listing_id, holding) in holdings.iter_mut() {
        if let Some(&price) = price_map.get(listing_id) {
            let total_cost = total_costs.get(listing_id).unwrap();
            let market_value = price * holding.quantity;

            holding.current_price = price;
            holding.market_value = market_value;
            holding.unrealised_gain = market_value - total_cost;
        }
    }

    let mut res: Vec<Holding> = holdings.into_values().collect();
    res.sort_by(|a, b| a.ticker.cmp(&b.ticker));
    Ok(res)
}
