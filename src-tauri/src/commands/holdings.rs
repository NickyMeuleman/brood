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
    avg_cost_basis_with_fees: Decimal,
    current_price: Decimal,
    market_value: Decimal,
    unrealised_gain: Decimal,
    unrealised_gain_with_fees: Decimal,
    // percentage_gain = unrealised_gain / total_cost.
    // Mathematically equal to (current_price - avg_cost_basis) / avg_cost_basis, because quantity cancels out.
    // Computed in Rust to avoid frontend arithmetic
    percentage_gain: Decimal,
    percentage_gain_with_fees: Decimal,
    total_fees: Decimal,
}

#[derive(Debug, sqlx::FromRow)]
struct TradeFeeRow {
    trade_id: i64,
    trade_qty: String,
    fee_amount: String,
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
  l.source_trade_id AS "source_trade_id?",
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

    // One row per trade_fee for trades that produced open lots.
    // CA-originated lots have no source_trade_id and receive no fee attribution
    // their cost basis is fully captured in price_per_unit at CA processing time.
    let fee_rows = sqlx::query_as!(
        TradeFeeRow,
        r#"
    SELECT
        t.id       AS "trade_id!",
        t.quantity AS "trade_qty!",
        tf.amount  AS "fee_amount!"
    FROM trade t
    JOIN trade_fee tf ON tf.trade_id = t.id
    WHERE t.id IN (
        SELECT source_trade_id
        FROM lot
        WHERE id NOT IN (SELECT lot_id FROM lot_close)
          AND source_trade_id IS NOT NULL
    )
    "#
    )
    .fetch_all(&db.pool)
    .await
    .map_err(AppError::from)?;

    // Sum sold quantity per lot
    // lot_id -> sold_qty
    let mut lot_sales = HashMap::new();
    for sale in sales {
        let qty = Decimal::from_str(&sale.quantity)
            .map_err(|_| AppError::Database("Malformed sell allocation data".to_string()))?;
        let qty_sold = lot_sales.entry(sale.origin_lot_id).or_insert(Decimal::ZERO);
        *qty_sold += qty;
    }

    // Sum fees per trade.
    // trade_id → (trade_qty, total_fees)
    // trade_qty is the same for every fee row of a given trade;
    // storing it here avoids a separate trade query.
    let mut fee_by_trade = HashMap::new();
    for r in fee_rows {
        let trade_qty = Decimal::from_str(&r.trade_qty)
            .map_err(|_| AppError::Database("Malformed trade quantity".to_string()))?;
        let fee = Decimal::from_str(&r.fee_amount)
            .map_err(|_| AppError::Database("Malformed trade fee amount".to_string()))?;
        let entry = fee_by_trade
            .entry(r.trade_id)
            .or_insert((trade_qty, Decimal::ZERO));
        entry.1 += fee;
    }

    // Aggregate lots into holdings keyed by listing_id.
    //
    // Two parallel cost accumulators are kept per holding:
    //   total_cost           : pure price × remaining qty, no fees
    //   total_cost_with_fees : above + proportional fee attribution
    //
    // Fees are attributed proportionally to remaining quantity:
    //   lot_fee = trade_total_fees x (lot_remaining / trade_qty)
    //
    // Accumulators are kept separately from avg_cost_basis to avoid
    // precision loss: avg_cost_basis = total_cost / quantity involves
    // a division that may produce a repeating Decimal. Recomputing
    // total_cost = avg_cost_basis x quantity would lose that precision.
    // unrealised_gain is always computed from the accumulator, never
    // from avg_cost_basis x quantity.
    let mut holdings = HashMap::new();
    let mut total_costs = HashMap::new();
    let mut total_costs_with_fees = HashMap::new();
    let mut total_fees_by_listing = HashMap::new();

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

        let lot_fee = row
            .source_trade_id
            .and_then(|tid| fee_by_trade.get(&tid))
            .map(|(trade_qty, fees)| fees * (remaining / trade_qty))
            .unwrap_or(Decimal::ZERO);

        let holding = holdings.entry(row.listing_id).or_insert(Holding {
            isin: row.isin,
            name: row.name,
            ticker: row.ticker,
            exchange_mic: row.mic,
            instrument_type: row.instrument_type,
            currency_code: row.currency,
            quantity: Decimal::ZERO,
            avg_cost_basis: Decimal::ZERO,
            avg_cost_basis_with_fees: Decimal::ZERO,
            current_price: Decimal::ZERO,
            market_value: Decimal::ZERO,
            unrealised_gain: Decimal::ZERO,
            unrealised_gain_with_fees: Decimal::ZERO,
            percentage_gain: Decimal::ZERO,
            percentage_gain_with_fees: Decimal::ZERO,
            total_fees: Decimal::ZERO,
        });

        holding.quantity += remaining;
        debug_assert!(holding.quantity > Decimal::ZERO);

        *total_costs.entry(row.listing_id).or_insert(Decimal::ZERO) += lot_cost;
        *total_costs_with_fees
            .entry(row.listing_id)
            .or_insert(Decimal::ZERO) += lot_cost + lot_fee;
        *total_fees_by_listing
            .entry(row.listing_id)
            .or_insert(Decimal::ZERO) += lot_fee;

        let tc = total_costs[&row.listing_id];
        let tcf = total_costs_with_fees[&row.listing_id];

        holding.avg_cost_basis = tc / holding.quantity;
        holding.avg_cost_basis_with_fees = tcf / holding.quantity;
        holding.total_fees = total_fees_by_listing[&row.listing_id];
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

    // Apply prices and compute market value, gains, and percentages.
    for (listing_id, holding) in holdings.iter_mut() {
        if let Some(&price) = price_map.get(listing_id) {
            let tc = total_costs[listing_id];
            let tcf = total_costs_with_fees[listing_id];
            let market_value = price * holding.quantity;

            holding.current_price = price;
            holding.market_value = market_value;
            holding.unrealised_gain = market_value - tc;
            holding.unrealised_gain_with_fees = market_value - tcf;
            // Divide by total_cost, not avg_cost_basis × quantity, to avoid
            // reintroducing the precision loss from the avg_cost_basis division.
            holding.percentage_gain = holding.unrealised_gain / tc;
            holding.percentage_gain_with_fees = holding.unrealised_gain_with_fees / tcf;
        }
    }

    let mut res: Vec<Holding> = holdings.into_values().collect();
    res.sort_by(|a, b| a.ticker.cmp(&b.ticker));
    Ok(res)
}
