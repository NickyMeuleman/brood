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

    // local currency (currency_code)
    avg_cost_basis: Decimal,
    avg_cost_basis_with_fees: Decimal,
    current_price: Decimal,
    market_value: Decimal,
    unrealised_gain: Decimal,
    unrealised_gain_with_fees: Decimal,
    total_fees: Decimal,
    // Total cash outflow to acquire the current position, excluding fees.
    // This is the accumulator used for unrealised_gain and percentage_gain —
    // exposing it directly avoids reconstructing it from derived values.
    total_cost: Decimal,
    total_cost_with_fees: Decimal,

    // EUR (= local currency values for EUR holdings)
    // Converted at the current spot rate.
    // TODO: values that are aggregated over time are approximations — a correct historical-rate value requires the FX rate
    // at each lot's acquisition date, not the current spot rate. Acceptable for
    // display; not suitable for tax reporting.
    avg_cost_basis_eur: Decimal,
    avg_cost_basis_with_fees_eur: Decimal,
    market_value_eur: Decimal,
    unrealised_gain_eur: Decimal,
    unrealised_gain_with_fees_eur: Decimal,
    total_fees_eur: Decimal,
    total_cost_eur: Decimal,
    total_cost_with_fees_eur: Decimal,

    // percentage_gain = unrealised_gain / total_cost.
    // Mathematically equal to (current_price - avg_cost_basis) / avg_cost_basis, because quantity cancels out.
    percentage_gain: Decimal,
    percentage_gain_with_fees: Decimal,
}

/// How much of the portfolio is held in a given currency,
/// for the currency breakdown UI.
#[derive(Debug, Clone, Serialize, Type)]
pub struct CurrencyAllocation {
    currency_code: String,
    market_value: Decimal,
    market_value_eur: Decimal,
    /// market_value_eur / totals.market_value_eur
    weight: Decimal,
}

/// Portfolio-level aggregates in EUR.
/// Sent in the response envelope so the frontend never has to
/// reduce across rows — these are read directly by table footers
/// and the portfolio_weight column via table meta.
#[derive(Debug, Clone, Serialize, Type)]
pub struct HoldingsTotals {
    market_value_eur: Decimal,
    unrealised_gain_eur: Decimal,
    unrealised_gain_with_fees_eur: Decimal,
    total_fees_eur: Decimal,
    percentage_gain: Decimal,
    percentage_gain_with_fees: Decimal,
    total_cost_eur: Decimal,
    total_cost_with_fees_eur: Decimal,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct HoldingsResponse {
    holdings: Vec<Holding>,
    totals: HoldingsTotals,
    currency_breakdown: Vec<CurrencyAllocation>,
}

#[derive(Debug, sqlx::FromRow)]
struct TradeFeeRow {
    trade_id: i64,
    trade_qty: String,
    fee_amount: String,
}

#[tauri::command]
#[specta::specta]
pub async fn get_holdings(db: State<'_, Db>) -> Result<HoldingsResponse, AppError> {
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
  JOIN listing li ON li.id = l.listing_id
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

    // Latest FX rate to EUR per currency.
    // EUR itself is not in fx_rate; handled as rate = 1 at lookup time.
    let fx_rows = sqlx::query!(
        r#"
        SELECT fr.currency, fr.rate_to_eur
        FROM fx_rate fr
        WHERE fr.date = (
            SELECT MAX(fr2.date)
            FROM fx_rate fr2
            WHERE fr2.currency = fr.currency
        )
        "#
    )
    .fetch_all(&db.pool)
    .await
    .map_err(AppError::from)?;

    let fx_map: HashMap<String, Decimal> = fx_rows
        .into_iter()
        .map(|r| {
            let rate = Decimal::from_str(&r.rate_to_eur)
                .map_err(|_| AppError::Database("Malformed fx_rate rate_to_eur".to_string()))?;
            Ok((r.currency, rate))
        })
        .collect::<Result<_, AppError>>()?;

    let eur_rate = |currency: &str| -> Result<Decimal, AppError> {
        if currency == "EUR" {
            return Ok(Decimal::ONE);
        }
        fx_map
            .get(currency)
            .copied()
            .ok_or_else(|| AppError::Database(format!("No FX rate found for {currency}")))
    };

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
            total_fees: Decimal::ZERO,
            avg_cost_basis_eur: Decimal::ZERO,
            avg_cost_basis_with_fees_eur: Decimal::ZERO,
            market_value_eur: Decimal::ZERO,
            unrealised_gain_eur: Decimal::ZERO,
            unrealised_gain_with_fees_eur: Decimal::ZERO,
            total_fees_eur: Decimal::ZERO,
            percentage_gain: Decimal::ZERO,
            percentage_gain_with_fees: Decimal::ZERO,
            total_cost: Decimal::ZERO,
            total_cost_with_fees: Decimal::ZERO,
            total_cost_eur: Decimal::ZERO,
            total_cost_with_fees_eur: Decimal::ZERO,
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

    // Apply prices, compute gains, percentages, and EUR values.
    for (listing_id, holding) in holdings.iter_mut() {
        if let Some(&price) = price_map.get(listing_id) {
            let tc = total_costs[listing_id];
            debug_assert!(tc != Decimal::ZERO);
            let tcf = total_costs_with_fees[listing_id];
            debug_assert!(tcf != Decimal::ZERO);
            let tf = total_fees_by_listing[listing_id];
            let market_value = price * holding.quantity;
            let rate = eur_rate(&holding.currency_code)?;

            holding.current_price = price;
            holding.market_value = market_value;
            holding.unrealised_gain = market_value - tc;
            holding.unrealised_gain_with_fees = market_value - tcf;
            holding.total_fees = tf;
            // Divide by total_cost, not avg_cost_basis × quantity, to avoid
            // reintroducing the precision loss from the avg_cost_basis division.
            holding.percentage_gain = holding.unrealised_gain / tc;
            holding.percentage_gain_with_fees = holding.unrealised_gain_with_fees / tcf;

            holding.market_value_eur = market_value * rate;
            holding.unrealised_gain_eur = holding.unrealised_gain * rate;
            holding.unrealised_gain_with_fees_eur = holding.unrealised_gain_with_fees * rate;
            // TODO: avg_cost_basis_eur, avg_cost_basis_with_fees_eur, and total_fees_eur
            // are approximations — correct values require the FX rate at each lot's
            // acquisition date, not the current spot rate. Acceptable for display;
            // not suitable for tax reporting.
            holding.avg_cost_basis_eur = holding.avg_cost_basis * rate;
            holding.avg_cost_basis_with_fees_eur = holding.avg_cost_basis_with_fees * rate;
            holding.total_fees_eur = tf * rate;

            holding.total_cost = tc;
            holding.total_cost_with_fees = tcf;
            holding.total_cost_eur = tc * rate;
            holding.total_cost_with_fees_eur = tcf * rate;
        }
    }

    let mut holdings_vec: Vec<Holding> = holdings.into_values().collect();
    holdings_vec.sort_by(|a, b| a.ticker.cmp(&b.ticker));

    let mut totals = HoldingsTotals {
        market_value_eur: Decimal::ZERO,
        unrealised_gain_eur: Decimal::ZERO,
        unrealised_gain_with_fees_eur: Decimal::ZERO,
        total_fees_eur: Decimal::ZERO,
        percentage_gain: Decimal::ZERO,
        percentage_gain_with_fees: Decimal::ZERO,
        total_cost_eur: Decimal::ZERO,
        total_cost_with_fees_eur: Decimal::ZERO,
    };
    // currency_code → (market_value in that currency, market_value_eur)
    let mut currency_map: HashMap<String, (Decimal, Decimal)> = HashMap::new();

    for h in &holdings_vec {
        totals.market_value_eur += h.market_value_eur;
        totals.unrealised_gain_eur += h.unrealised_gain_eur;
        totals.unrealised_gain_with_fees_eur += h.unrealised_gain_with_fees_eur;
        totals.total_fees_eur += h.total_fees_eur;
        totals.total_cost_eur += h.total_cost_eur;
        totals.total_cost_with_fees_eur += h.total_cost_with_fees_eur;

        let entry = currency_map
            .entry(h.currency_code.clone())
            .or_insert((Decimal::ZERO, Decimal::ZERO));
        entry.0 += h.market_value;
        entry.1 += h.market_value_eur;
    }
    totals.percentage_gain = totals.unrealised_gain_eur / totals.total_cost_eur;
    totals.percentage_gain_with_fees =
        totals.unrealised_gain_with_fees_eur / totals.total_cost_with_fees_eur;

    let total_mv_eur = totals.market_value_eur;
    let mut currency_breakdown: Vec<CurrencyAllocation> = currency_map
        .into_iter()
        .map(
            |(currency_code, (market_value, market_value_eur))| CurrencyAllocation {
                currency_code,
                market_value,
                market_value_eur,
                weight: if total_mv_eur.is_zero() {
                    Decimal::ZERO
                } else {
                    market_value_eur / total_mv_eur
                },
            },
        )
        .collect();
    currency_breakdown.sort_by(|a, b| b.market_value_eur.cmp(&a.market_value_eur));

    Ok(HoldingsResponse {
        holdings: holdings_vec,
        totals,
        currency_breakdown,
    })
}
