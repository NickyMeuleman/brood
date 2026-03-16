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
    total_cost: Decimal,
    total_cost_with_fees: Decimal,

    // EUR equivalents, (= local currency values for EUR holdings)
    // TODO: time-aggregated values (avg_cost_basis_eur etc.) are approximations —
    // correct values require the FX rate at each lot's acquisition date.
    // Acceptable for display; not suitable for tax reporting.
    avg_cost_basis_eur: Decimal,
    avg_cost_basis_with_fees_eur: Decimal,
    current_price_eur: Decimal,
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
    // Fees paid as a fraction of acquisition cost (fees / total_cost).
    // Measures drag at time of investing, independent of subsequent price movement.
    // No _with_fees variant — using total_cost_with_fees as denominator would be circular.
    fee_drag: Decimal,
}

/// How much of the portfolio is held in a given currency, for the currency breakdown UI.
#[derive(Debug, Clone, Serialize, Type)]
pub struct CurrencyAllocation {
    currency_code: String,
    market_value: Decimal,
    market_value_eur: Decimal,
    /// market_value_eur / totals.market_value_eur
    weight: Decimal,
}

/// Portfolio-level aggregates in EUR.
/// Sent in the response envelope so the frontend never has to reduce across rows —
/// read directly by table footers and the portfolio_weight column via table meta.
#[derive(Debug, Clone, Serialize, Type, Default)]
pub struct HoldingsTotals {
    market_value_eur: Decimal,
    unrealised_gain_eur: Decimal,
    unrealised_gain_with_fees_eur: Decimal,
    total_fees_eur: Decimal,
    percentage_gain: Decimal,
    percentage_gain_with_fees: Decimal,
    total_cost_eur: Decimal,
    total_cost_with_fees_eur: Decimal,
    fee_drag: Decimal,
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

struct AllLotRow {
    lot_id: i64,
    parent_lot_id: Option<i64>,
    source_trade_id: Option<i64>,
    qty: String,
}

fn parse_decimal(s: &str, ctx: &str) -> Result<Decimal, AppError> {
    Decimal::from_str(s).map_err(|_| AppError::Database(format!("Malformed {ctx}: {s}")))
}

/// Computes the fee basis attributable to every lot at its full `qty_at_acquisition`.
///
/// Fees follow shares proportionally at every CA transformation:
///
/// - Trade lot:  `total_trade_fees × (lot_qty / trade_qty)`
///   Handles partial fills where one trade produces multiple lots.
///
/// - CA lot:     `parent_fee_basis × (lot_qty / sum_of_sibling_qtys)`
///   Propagates fees down the tree. Handles splits, spinoffs, and chains of
///   arbitrary depth without ever needing to walk upward to the root.
///
/// Requires `all_lots` in topological order (parents before children).
/// `ORDER BY id ASC` guarantees this because a lot is always inserted after its parent.
fn compute_fee_basis(
    all_lots: &[AllLotRow],
    fee_by_trade: &HashMap<i64, (Decimal, Decimal)>,
) -> Result<HashMap<i64, Decimal>, AppError> {
    // Pass 1 — sum child qtys per parent for proportional distribution.
    let mut sibling_total_qty: HashMap<i64, Decimal> = HashMap::new();
    for lot in all_lots {
        if let Some(parent_id) = lot.parent_lot_id {
            let qty = parse_decimal(&lot.qty, "lot qty_at_acquisition")?;
            *sibling_total_qty.entry(parent_id).or_default() += qty;
        }
    }

    // Pass 2 — propagate fee basis downward from roots to leaves.
    let mut fee_basis: HashMap<i64, Decimal> = HashMap::new();
    for lot in all_lots {
        let qty = parse_decimal(&lot.qty, "lot qty_at_acquisition")?;

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

    Ok(fee_basis)
}

#[tauri::command]
#[specta::specta]
pub async fn get_holdings(db: State<'_, Db>) -> Result<HoldingsResponse, AppError> {
    // 1. All lots (open and closed) for fee propagation.
    //    ORDER BY id ASC guarantees topological order — a lot is always inserted
    //    after its parent, so parents always appear before children after sorting.
    let all_lots = sqlx::query_as!(
        AllLotRow,
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
            Ok((
                r.currency,
                parse_decimal(&r.rate_to_eur, "fx_rate rate_to_eur")?,
            ))
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
        let trade_qty = parse_decimal(&r.trade_qty, "trade quantity")?;
        let fee = parse_decimal(&r.fee_amount, "trade fee amount")?;
        let entry = fee_by_trade
            .entry(r.trade_id)
            .or_insert((trade_qty, Decimal::ZERO));
        entry.1 += fee;
    }

    // 7. Propagate fee basis through the lot tree.
    let fee_basis = compute_fee_basis(&all_lots, &fee_by_trade)?;

    // 8. Sum sold quantity per lot.  lot_id → total_sold_qty.
    let mut lot_sales: HashMap<i64, Decimal> = HashMap::new();
    for sale in sales {
        let qty = parse_decimal(&sale.quantity, "sell allocation quantity")?;
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
        let initial = parse_decimal(&row.qty, "lot qty_at_acquisition")?;
        let sold = lot_sales.get(&row.lot_id).copied().unwrap_or(Decimal::ZERO);
        let remaining = initial - sold;

        if remaining <= Decimal::ZERO {
            continue;
        }

        let lot_price = parse_decimal(&row.price, "lot price_per_unit")?;
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
            avg_cost_basis_eur: Decimal::ZERO,
            avg_cost_basis_with_fees: Decimal::ZERO,
            avg_cost_basis_with_fees_eur: Decimal::ZERO,
            current_price: Decimal::ZERO,
            current_price_eur: Decimal::ZERO,
            market_value: Decimal::ZERO,
            market_value_eur: Decimal::ZERO,
            unrealised_gain: Decimal::ZERO,
            unrealised_gain_eur: Decimal::ZERO,
            unrealised_gain_with_fees: Decimal::ZERO,
            unrealised_gain_with_fees_eur: Decimal::ZERO,
            total_fees: Decimal::ZERO,
            total_fees_eur: Decimal::ZERO,
            total_cost: Decimal::ZERO,
            total_cost_eur: Decimal::ZERO,
            total_cost_with_fees: Decimal::ZERO,
            total_cost_with_fees_eur: Decimal::ZERO,
            percentage_gain: Decimal::ZERO,
            percentage_gain_with_fees: Decimal::ZERO,
            fee_drag: Decimal::ZERO,
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
            Ok((
                r.listing_id,
                parse_decimal(&r.close, "price_history close")?,
            ))
        })
        .collect::<Result<_, AppError>>()?;

    // 11. Apply prices, compute gains, percentages, and EUR values.
    for (listing_id, holding) in holdings.iter_mut() {
        let Some(&price) = price_map.get(listing_id) else {
            continue;
        };

        let tc = total_costs[listing_id];
        debug_assert!(tc != Decimal::ZERO);
        let tcf = total_costs_with_fees[listing_id];
        debug_assert!(tcf != Decimal::ZERO);
        let tf = total_fees_by_listing[listing_id];
        let market_value = price * holding.quantity;
        let rate = eur_rate(&holding.currency_code)?;

        holding.current_price = price;
        holding.current_price_eur = price * rate;
        holding.market_value = market_value;
        holding.market_value_eur = market_value * rate;
        holding.total_fees = tf;
        holding.total_fees_eur = tf * rate;
        holding.total_cost = tc;
        holding.total_cost_eur = tc * rate;
        holding.total_cost_with_fees = tcf;
        holding.total_cost_with_fees_eur = tcf * rate;
        holding.unrealised_gain = market_value - tc;
        holding.unrealised_gain_eur = holding.unrealised_gain * rate;
        holding.unrealised_gain_with_fees = market_value - tcf;
        holding.unrealised_gain_with_fees_eur = holding.unrealised_gain_with_fees * rate;
        // Divide by total_cost accumulator, not avg_cost_basis × quantity, to
        // avoid reintroducing the precision loss from the avg_cost_basis division.
        holding.percentage_gain = holding.unrealised_gain / tc;
        holding.percentage_gain_with_fees = holding.unrealised_gain_with_fees / tcf;
        holding.fee_drag = tf / tc;

        holding.avg_cost_basis_eur = holding.avg_cost_basis * rate;
        holding.avg_cost_basis_with_fees_eur = holding.avg_cost_basis_with_fees * rate;
    }

    // 12. Sort, then build response envelope with portfolio totals and currency breakdown.
    let mut holdings_vec: Vec<Holding> = holdings.into_values().collect();
    holdings_vec.sort_by(|a, b| a.ticker.cmp(&b.ticker));

    let mut totals = HoldingsTotals::default();
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
    totals.fee_drag = totals.total_fees_eur / totals.total_cost_eur;

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
