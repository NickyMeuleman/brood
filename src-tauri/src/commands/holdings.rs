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
    // Total amount paid for shares currently held
    total_cost: Decimal,
    total_cost_with_fees: Decimal,
    // Fees paid as a fraction of acquisition cost.
    // measures the drag at time of investing, independent of subsequent price movement.
    // No _with_fees variant needed as fee drag is always fees / cost_without_fees.
    // Using total_cost_with_fees as denominator would be circular.
    fee_drag: Decimal,

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
    // 1. All lots (open and closed) — needed to propagate fees through CA chains.
    //    Ordered by id ASC, which is topological order: a lot is always inserted
    //    after its parent, so parents are guaranteed to appear before children.
    let all_lots = sqlx::query!(
        r#"
        SELECT
            id              AS "lot_id!",
            parent_lot_id   AS "parent_lot_id?",
            source_trade_id AS "source_trade_id?",
            qty_at_acquisition AS "qty!"
        FROM lot
        ORDER BY id ASC
        "#
    )
    .fetch_all(&db.pool)
    .await
    .map_err(AppError::from)?;

    // 2. Open lots with instrument and listing context — for aggregation only.
    let open_lots = sqlx::query!(
        r#"
        SELECT
            l.id                  AS "lot_id!",
            l.listing_id          AS "listing_id!",
            l.qty_at_acquisition  AS "qty!",
            l.price_per_unit      AS "price!",
            l.price_currency_code AS "currency!",
            i.isin                AS "isin!",
            i.name                AS "name!",
            i.instrument_type     AS "instrument_type!: InstrumentType",
            li.ticker             AS "ticker!",
            li.exchange_mic       AS "mic!"
        FROM lot l
        JOIN instrument i ON l.instrument_id = i.id
        JOIN listing li   ON li.id = l.listing_id
        WHERE l.id NOT IN (SELECT lot_id FROM lot_close)
        "#
    )
    .fetch_all(&db.pool)
    .await
    .map_err(AppError::from)?;

    // 3. All sell allocations against open lots.
    let sales = sqlx::query_as!(
        SellAllocation,
        r#"
        SELECT * FROM sell_allocation sa
        WHERE sa.origin_lot_id NOT IN (SELECT lot_id FROM lot_close)
        "#
    )
    .fetch_all(&db.pool)
    .await
    .map_err(AppError::from)?;

    // 4. All trade fees.
    let fee_rows = sqlx::query_as!(
        TradeFeeRow,
        r#"
        SELECT
            t.id       AS "trade_id!",
            t.quantity AS "trade_qty!",
            tf.amount  AS "fee_amount!"
        FROM trade t
        JOIN trade_fee tf ON tf.trade_id = t.id
        "#
    )
    .fetch_all(&db.pool)
    .await
    .map_err(AppError::from)?;

    // 5. Latest FX rate to EUR per currency.
    //    EUR itself is not in fx_rate; handled as rate = 1 at lookup time.
    let fx_rows = sqlx::query!(
        r#"
        SELECT fr.currency, fr.rate_to_eur
        FROM fx_rate fr
        WHERE fr.date = (
            SELECT MAX(fr2.date) FROM fx_rate fr2
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

    // 6. Sum fees per trade.  trade_id → (trade_qty, total_fees).
    //    trade_qty is the same across all fee rows for a given trade;
    //    storing it here avoids a separate trade lookup later.
    let mut fee_by_trade: HashMap<i64, (Decimal, Decimal)> = HashMap::new();
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

    // 7. Precompute fee_basis[lot_id]: fees attributable to each lot at full qty.
    //
    //    Fees follow shares proportionally at every event:
    //
    //      Trade lot   fees × (lot_qty / trade_qty)
    //                  Handles partial fills — multiple lots from one trade each
    //                  receive their proportional share.
    //
    //      CA lot      parent_fee_basis × (lot_qty / sum_of_sibling_qtys)
    //                  Handles splits (1 child → ratio = 1), spinoffs (N children),
    //                  and chains of arbitrary depth by propagating through the tree.
    //
    //    Requires topological order (parents before children) — guaranteed by
    //    ORDER BY id ASC in query 1.

    // First pass: sum child qtys per parent so we can prorate across siblings.
    let mut sibling_total_qty: HashMap<i64, Decimal> = HashMap::new();
    for lot in &all_lots {
        if let Some(parent_id) = lot.parent_lot_id {
            let qty = Decimal::from_str(&lot.qty)
                .map_err(|_| AppError::Database("Malformed lot qty_at_acquisition".to_string()))?;
            *sibling_total_qty.entry(parent_id).or_default() += qty;
        }
    }

    // Second pass: walk from roots to leaves, propagating fees downward.
    let mut fee_basis: HashMap<i64, Decimal> = HashMap::new();
    for lot in &all_lots {
        let qty = Decimal::from_str(&lot.qty)
            .map_err(|_| AppError::Database("Malformed lot qty_at_acquisition".to_string()))?;

        let basis = match (lot.source_trade_id, lot.parent_lot_id) {
            (Some(tid), _) => fee_by_trade
                .get(&tid)
                .map(|(trade_qty, fees)| fees * (qty / trade_qty))
                .unwrap_or(Decimal::ZERO),

            (None, Some(parent_id)) => {
                let parent_basis = fee_basis.get(&parent_id).copied().unwrap_or(Decimal::ZERO);
                let sibling_qty = sibling_total_qty[&parent_id];
                parent_basis * (qty / sibling_qty)
            }

            (None, None) => Decimal::ZERO,
        };

        fee_basis.insert(lot.lot_id, basis);
    }

    // 8. Sum sold quantity per lot.  lot_id → total_sold_qty.
    let mut lot_sales: HashMap<i64, Decimal> = HashMap::new();
    for sale in sales {
        let qty = Decimal::from_str(&sale.quantity)
            .map_err(|_| AppError::Database("Malformed sell allocation quantity".to_string()))?;
        *lot_sales.entry(sale.origin_lot_id).or_default() += qty;
    }

    // 9. Aggregate open lots into holdings keyed by listing_id.
    //
    //    Two parallel cost accumulators per holding:
    //      total_cost           — price × remaining qty, no fees
    //      total_cost_with_fees — above + proportional fee attribution
    //
    //    lot_fee = fee_basis[lot_id] × (remaining / initial)
    //    Prorating by remaining correctly scales down fees after partial sells.
    //
    //    Accumulators are kept separate from avg_cost_basis to avoid precision
    //    loss: avg_cost_basis = total_cost / quantity involves a division that
    //    may produce a repeating Decimal. Recomputing total_cost from
    //    avg_cost_basis × quantity would lose that precision.
    let mut holdings: HashMap<i64, Holding> = HashMap::new();
    let mut total_costs: HashMap<i64, Decimal> = HashMap::new();
    let mut total_costs_with_fees: HashMap<i64, Decimal> = HashMap::new();
    let mut total_fees_by_listing: HashMap<i64, Decimal> = HashMap::new();

    for row in open_lots {
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

        let lot_fee = fee_basis
            .get(&row.lot_id)
            .map(|basis| basis * (remaining / initial))
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
            total_cost: Decimal::ZERO,
            total_cost_with_fees: Decimal::ZERO,
            fee_drag: Decimal::ZERO,
            avg_cost_basis_eur: Decimal::ZERO,
            avg_cost_basis_with_fees_eur: Decimal::ZERO,
            market_value_eur: Decimal::ZERO,
            unrealised_gain_eur: Decimal::ZERO,
            unrealised_gain_with_fees_eur: Decimal::ZERO,
            total_fees_eur: Decimal::ZERO,
            total_cost_eur: Decimal::ZERO,
            total_cost_with_fees_eur: Decimal::ZERO,
            percentage_gain: Decimal::ZERO,
            percentage_gain_with_fees: Decimal::ZERO,
        });

        holding.quantity += remaining;
        debug_assert!(holding.quantity > Decimal::ZERO);

        *total_costs.entry(row.listing_id).or_default() += lot_cost;
        *total_costs_with_fees.entry(row.listing_id).or_default() += lot_cost + lot_fee;
        *total_fees_by_listing.entry(row.listing_id).or_default() += lot_fee;

        let tc = total_costs[&row.listing_id];
        let tcf = total_costs_with_fees[&row.listing_id];
        holding.avg_cost_basis = tc / holding.quantity;
        holding.avg_cost_basis_with_fees = tcf / holding.quantity;
    }

    // 10. Latest price per listing.
    //     PERF: future enhancement — only fetch prices for listings in holdings.
    let prices = sqlx::query!(
        r#"
        SELECT ph.listing_id, ph.close
        FROM price_history ph
        WHERE ph.date = (
            SELECT MAX(ph2.date) FROM price_history ph2
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
                .map_err(|_| AppError::Database("Malformed price_history close".to_string()))?;
            Ok((r.listing_id, price))
        })
        .collect::<Result<_, AppError>>()?;

    // 11. Apply prices, compute gains, percentages, and EUR values.
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
            // Divide by total_cost accumulator, not avg_cost_basis × quantity, to
            // avoid reintroducing the precision loss from the avg_cost_basis division.
            holding.percentage_gain = holding.unrealised_gain / tc;
            holding.percentage_gain_with_fees = holding.unrealised_gain_with_fees / tcf;

            holding.market_value_eur = market_value * rate;
            holding.unrealised_gain_eur = holding.unrealised_gain * rate;
            holding.unrealised_gain_with_fees_eur = holding.unrealised_gain_with_fees * rate;
            holding.avg_cost_basis_eur = holding.avg_cost_basis * rate;
            holding.avg_cost_basis_with_fees_eur = holding.avg_cost_basis_with_fees * rate;
            holding.total_fees_eur = tf * rate;
            holding.fee_drag = tf / tc;
            holding.total_cost = tc;
            holding.total_cost_with_fees = tcf;
            holding.total_cost_eur = tc * rate;
            holding.total_cost_with_fees_eur = tcf * rate;
        }
    }

    // 12. Sort, then build response envelope with portfolio totals and currency breakdown.
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
