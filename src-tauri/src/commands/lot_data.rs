use crate::AppError;
use crate::commands::CurrencyPair;
use crate::commands::get_rates;
use crate::commands::latest_on_or_before;
use crate::db::types::InstrumentType;
use crate::parse_decimal_internal;
use chrono::{NaiveDate, NaiveDateTime};
use rust_decimal::Decimal;
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;

/// Everything known about a lot that is independent of any specific date.
#[derive(Debug, Clone)]
pub struct LotRecord {
    // --- identity (used by get_holdings for table rows) ---
    pub id: i64,
    pub listing_id: i64,
    pub instrument_id: i64,
    pub isin: String,
    pub name: String,
    pub ticker: String,
    pub exchange_mic: String,
    pub instrument_type: InstrumentType,
    pub currency_code: String,

    // --- lot facts ---
    pub qty_at_acquisition: Decimal,
    pub price_per_unit: Decimal, // in listing currency

    // --- custody ---
    pub broker_id_at_acquisition: i64,
    /// Sorted by ascending date: (date, to broker_id)
    pub transfers: Vec<(NaiveDate, i64)>,

    // --- timeline ---
    /// When this lot first became an active position.
    /// Trade lots: `trade.executed_at`. CA lots: `corporate_action.effective_date`.
    pub existence_start: NaiveDate,
    /// Date this lot was retired by a corporate action (exclusive upper bound).
    /// `None` for lots that are still open.
    pub close_date: Option<NaiveDate>,

    // --- cost basis ---
    /// Original trade execution date, propagated up the `parent_lot_id` chain.
    /// This is the date used to look up the correct historical FX rate for
    /// cost-basis calculations, even for post-split CA lots.
    pub acquisition_datetime: NaiveDateTime,
    /// EUR/listing_currency rate on `acquisition_date`. Already resolved
    /// so callers never need to touch the FX table for cost calculations.
    pub acquisition_fx_rate: Decimal,

    // --- fees for the full lot quantity ---
    /// Scale by `qty / qty_at_acquisition` to get fees for any partial qty.
    /// `.local` is in listing currency; `.eur` is in EUR.
    pub fees: CurrencyPair<Decimal>,

    // --- sell history, sorted ascending by sell date ---
    pub sells: Vec<(NaiveDate, Decimal)>,
}

impl LotRecord {
    /// Shares remaining as of the close of `date`.
    pub fn qty_remaining_at(&self, date: NaiveDate) -> Decimal {
        let sold: Decimal = self
            .sells
            .iter()
            // `sells` is sorted ascending by date, so take_while is safe and efficient.
            .take_while(|(d, _)| *d <= date)
            .map(|(_, q)| *q)
            .sum();
        self.qty_at_acquisition - sold
    }

    /// `true` if this lot represents a non-zero position on `date`.
    pub fn is_active_at(&self, date: NaiveDate) -> bool {
        self.existence_start <= date
            && self.close_date.map_or(true, |c| c > date)
            && self.qty_remaining_at(date) > Decimal::ZERO
    }

    /// EUR cost contribution for `qty` shares from this lot.
    /// Uses the historical acquisition FX rate
    pub fn cost_contribution_eur(&self, qty: Decimal) -> Decimal {
        qty * self.price_per_unit * self.acquisition_fx_rate
    }

    /// Fees in EUR attributed to `qty` shares from this lot.
    pub fn fees_eur_for_qty(&self, qty: Decimal) -> Decimal {
        if self.qty_at_acquisition.is_zero() {
            return Decimal::ZERO;
        }
        self.fees.eur * (qty / self.qty_at_acquisition)
    }

    /// Fees in listing currency attributed to `qty` shares from this lot.
    pub fn fees_local_for_qty(&self, qty: Decimal) -> Decimal {
        if self.qty_at_acquisition.is_zero() {
            return Decimal::ZERO;
        }
        self.fees.local * (qty / self.qty_at_acquisition)
    }

    /// broker_id after transfers (if any)
    /// transfers has to be sorted chronologically
    pub fn broker_id_as_of(&self, date: NaiveDate) -> i64 {
        self.transfers
            .iter()
            .rev()
            .find(|(d, _)| *d <= date)
            .map(|(_, b)| *b)
            .unwrap_or(self.broker_id_at_acquisition)
    }
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

/// Load and pre-process every lot into a [`LotRecord`].
///
/// Performs all multi-table joins, the `parent_lot_id` traversal for fee and
/// date propagation, and the historical FX lookups that both `get_holdings` and
/// `get_portfolio_history` require. Callers receive a flat `Vec<LotRecord>` and
/// apply date-based filtering with [`LotRecord::is_active_at`].
pub async fn load_lot_records(pool: &Pool<Sqlite>) -> Result<Vec<LotRecord>, AppError> {
    // ---- All lots -------------------------------------------------------
    // ORDER BY l.id ASC is vital for handling child CA lots topologically!
    // all lots, needed because some open lots (CA-originated lots) lack info about the time of the
    // price_per_unit at original acquisition time.
    // This info is needed to build a historically correct unit_price_basis_eur
    // that uses FX rates of each moment that lot's shares were bought
    // qty_at_acquisition and price_per_unit are needed for accurate fee proportional attribution
    let raw_lots = sqlx::query!(
        r#"
        SELECT
            l.id AS lot_id,
            l.listing_id,
            l.instrument_id,
            l.broker_id_at_acquisition,
            l.parent_lot_id,
            l.source_trade_id, 
            l.source_ca_id,
            l.qty_at_acquisition,
            l.price_per_unit,
            t.executed_at AS "trade_executed_at: NaiveDateTime",
            li.currency_code,
            li.ticker,
            li.exchange_mic,
            i.isin,
            i.name,
            i.instrument_type AS "instrument_type: InstrumentType"
        FROM lot l
        JOIN listing li ON li.id = l.listing_id
        JOIN instrument i ON i.id = l.instrument_id
        LEFT JOIN trade t ON t.id = l.source_trade_id
        ORDER BY l.id ASC
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::from)?;

    if raw_lots.is_empty() {
        return Ok(vec![]);
    }

    // ---- lot_close dates ------------------------------------------------
    let lot_close_dates: HashMap<i64, NaiveDate> = sqlx::query!(
        r#"
        SELECT
            lot_id,
            closed_at AS "closed_at: NaiveDateTime"
        FROM lot_close
        "#
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| (r.lot_id, r.closed_at.date()))
    .collect();

    // ---- lot_transfer history (for per-broker FIFO scoping) --------
    let transfer_rows = sqlx::query!(
        r#"
        SELECT
            lot_id,
            to_broker_id,
            transferred_at AS "transferred_at: NaiveDateTime"
        FROM lot_transfer
        ORDER BY lot_id, transferred_at ASC
        "#
    )
    .fetch_all(pool)
    .await?;

    let mut transfers_by_lot: HashMap<i64, Vec<(NaiveDate, i64)>> = HashMap::new();
    for row in transfer_rows {
        transfers_by_lot
            .entry(row.lot_id)
            .or_default()
            .push((row.transferred_at.date(), row.to_broker_id));
    }

    // ---- CA effective dates (existence_start for CA-originated lots) -----
    let ca_start_dates: HashMap<i64, NaiveDate> = sqlx::query!(
        r#"
        SELECT
            l.id AS "lot_id!",
            ca.effective_date AS "effective_date!: NaiveDate"
        FROM lot l
        JOIN corporate_action ca ON ca.id = l.source_ca_id
        WHERE l.source_ca_id IS NOT NULL
        "#
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| (r.lot_id, r.effective_date))
    .collect();

    // ---- Full FX rate history -------------------------------------------
    let all_rates = get_rates(pool).await?;

    // ---- Trade fees -----------------------------------------------------
    let fee_rows = sqlx::query!(
        r#"
        SELECT
            t.id             AS "trade_id!",
            t.quantity       AS trade_qty,
            t.executed_at    AS "executed_at: NaiveDateTime",
            li.currency_code AS listing_currency,
            tf.amount        AS fee_amount,
            tf.currency_code AS fee_currency
        FROM trade t
        JOIN trade_fee tf ON tf.trade_id = t.id
        JOIN listing li   ON li.id       = t.listing_id
        "#
    )
    .fetch_all(pool)
    .await?;

    let mut fee_by_trade: HashMap<i64, TradeFeeAgg> = HashMap::new();
    for r in &fee_rows {
        let fee = parse_decimal_internal(&r.fee_amount, "fee amount")?;
        let trade_qty = parse_decimal_internal(&r.trade_qty, "trade qty")?;
        let date = r.executed_at.date();

        let fee_eur = if r.fee_currency == "EUR" {
            fee
        } else {
            let rate = all_rates
                .get(&r.fee_currency)
                .and_then(|m| latest_on_or_before(m, date))
                .ok_or_else(|| {
                    AppError::MissingData(format!(
                        "No FX rate for {} on {date} (fee currency)",
                        r.fee_currency
                    ))
                })?;
            fee * rate
        };

        let fee_local = if r.fee_currency == r.listing_currency {
            fee
        } else if r.listing_currency == "EUR" {
            fee_eur
        } else {
            let listing_rate = all_rates
                .get(&r.listing_currency)
                .and_then(|m| latest_on_or_before(m, date))
                .ok_or_else(|| {
                    AppError::MissingData(format!(
                        "No FX rate for {} on {date} (listing currency)",
                        r.listing_currency
                    ))
                })?;
            fee_eur / listing_rate
        };

        let entry = fee_by_trade.entry(r.trade_id).or_insert(TradeFeeAgg {
            // trade_qty is identical for every fee row of the same trade
            trade_qty,
            fees: CurrencyPair::default(),
        });
        entry.fees.local += fee_local;
        entry.fees.eur += fee_eur;
    }

    // ---- Sell allocations with dates ------------------------------------
    let sell_rows = sqlx::query!(
        r#"
        SELECT
            sa.origin_lot_id,
            sa.quantity,
            t.executed_at AS "sell_date!: NaiveDateTime"
        FROM sell_allocation sa
        JOIN trade t ON t.id = sa.sell_trade_id
        ORDER BY sa.origin_lot_id, t.executed_at
        "#
    )
    .fetch_all(pool)
    .await?;

    let mut sells_by_lot: HashMap<i64, Vec<(NaiveDate, Decimal)>> = HashMap::new();
    for row in sell_rows {
        let qty = parse_decimal_internal(&row.quantity, "sell quantity")?;
        sells_by_lot
            .entry(row.origin_lot_id)
            .or_default()
            .push((row.sell_date.date(), qty));
    }

    // ---- Traverse lots in ascending ID order ----------------------------
    //
    // Parent lots always have lower IDs than their CA children, so ascending
    // ID order guarantees every parent is resolved before its children.
    //
    // For each lot we compute:
    //   acquisition_date  — the original trade date, propagated upward
    //   fees              — attributed portion of trade fees, split by cost ratio
    let mut acquisition_datetimes: HashMap<i64, NaiveDateTime> = HashMap::new();
    let mut lot_fees: HashMap<i64, CurrencyPair<Decimal>> = HashMap::new();
    let mut lot_costs: HashMap<i64, Decimal> = HashMap::new();

    for r in &raw_lots {
        let qty = parse_decimal_internal(&r.qty_at_acquisition, "lot qty")?;
        let price = parse_decimal_internal(&r.price_per_unit, "lot price")?;
        let cost = qty * price;
        lot_costs.insert(r.lot_id, cost);

        let (acquisition_datetime, fees) = match (r.source_trade_id, r.parent_lot_id) {
            // Trade-originated lot: use the trade's execution date directly.
            (Some(trade_id), _) => {
                let datetime = r.trade_executed_at.ok_or_else(|| {
                    AppError::MissingData(format!(
                        "Lot {} has source_trade_id but missing trade executed_at",
                        r.lot_id
                    ))
                })?;

                let fees = fee_by_trade
                    .get(&trade_id)
                    .map(|agg| {
                        let ratio = if agg.trade_qty.is_zero() {
                            Decimal::ZERO
                        } else {
                            qty / agg.trade_qty
                        };
                        CurrencyPair {
                            local: agg.fees.local * ratio,
                            eur: agg.fees.eur * ratio,
                        }
                    })
                    .unwrap_or_default();

                (datetime, fees)
            }
            // CA-originated lot: inherit date and fees from the parent,
            // proportioned by the ratio of this lot's cost to the parent's cost.
            (None, Some(parent_id)) => {
                let parent_datetime = *acquisition_datetimes.get(&parent_id).ok_or_else(|| {
                    AppError::Internal(format!(
                        "Parent lot {parent_id} not yet processed before child lot {}",
                        r.lot_id
                    ))
                })?;
                let parent_cost = *lot_costs.get(&parent_id).ok_or_else(|| {
                    AppError::Internal(format!("Cost missing for parent lot {parent_id}"))
                })?;
                let parent_fees = *lot_fees.get(&parent_id).ok_or_else(|| {
                    AppError::Internal(format!("Fees missing for parent lot {parent_id}"))
                })?;

                let ratio = if parent_cost.is_zero() {
                    Decimal::ZERO
                } else {
                    cost / parent_cost
                };

                let fees = CurrencyPair {
                    local: parent_fees.local * ratio,
                    eur: parent_fees.eur * ratio,
                };

                (parent_datetime, fees)
            }

            (None, None) => {
                return Err(AppError::Internal(format!(
                    "Lot {} has neither source_trade_id nor parent_lot_id",
                    r.lot_id
                )));
            }
        };

        acquisition_datetimes.insert(r.lot_id, acquisition_datetime);
        lot_fees.insert(r.lot_id, fees);
    }

    // ---- Build LotRecords -----------------------------------------------
    let mut records = Vec::with_capacity(raw_lots.len());

    for r in &raw_lots {
        let acquisition_datetime = acquisition_datetimes[&r.lot_id];
        let acquisition_date = acquisition_datetime.date();

        let acquisition_fx_rate = if r.currency_code == "EUR" {
            Decimal::ONE
        } else {
            all_rates
                .get(&r.currency_code)
                .and_then(|m| latest_on_or_before(m, acquisition_date))
                .ok_or_else(|| {
                    AppError::MissingData(format!(
                        "No FX rate for {} on {acquisition_date} (lot {} acquisition)",
                        r.currency_code, r.lot_id
                    ))
                })?
        };

        // Trade lots exist from their trade execution date.
        // CA lots exist from the corporate action's effective date.
        let existence_start = if r.source_trade_id.is_some() {
            acquisition_date
        } else {
            *ca_start_dates.get(&r.lot_id).ok_or_else(|| {
                AppError::Internal(format!(
                    "CA lot {} has no corporate_action effective_date",
                    r.lot_id
                ))
            })?
        };

        records.push(LotRecord {
            id: r.lot_id,
            listing_id: r.listing_id,
            instrument_id: r.instrument_id,
            isin: r.isin.clone(),
            name: r.name.clone(),
            ticker: r.ticker.clone(),
            exchange_mic: r.exchange_mic.clone(),
            instrument_type: r.instrument_type.clone(),
            currency_code: r.currency_code.clone(),
            qty_at_acquisition: parse_decimal_internal(
                &r.qty_at_acquisition,
                "qty_at_acquisition",
            )?,
            price_per_unit: parse_decimal_internal(&r.price_per_unit, "price_per_unit")?,
            broker_id_at_acquisition: r.broker_id_at_acquisition,
            transfers: transfers_by_lot.remove(&r.lot_id).unwrap_or_default(),
            existence_start,
            close_date: lot_close_dates.get(&r.lot_id).copied(),
            acquisition_datetime,
            acquisition_fx_rate,
            fees: lot_fees[&r.lot_id],
            sells: sells_by_lot.remove(&r.lot_id).unwrap_or_default(),
        });
    }

    Ok(records)
}

/// Sorts `candidates` by (acquisition date, lot id) ascending and greedily
/// consumes `quantity` from oldest first.
/// broker agnostic, callers should enforce `candidates` belong to the same broker
/// Returns vec of (affected lot_id, amount of that lots units sold)
pub fn match_fifo_lots(
    candidates: &[&LotRecord],
    quantity: Decimal,
    as_of: NaiveDate,
) -> Result<Vec<(i64, Decimal)>, AppError> {
    let mut sorted: Vec<&LotRecord> = candidates.to_vec();
    sorted.sort_by_key(|l| (l.acquisition_datetime.date(), l.id));

    let mut remaining = quantity;
    let mut allocations = Vec::new();
    for lot in sorted {
        if remaining.is_zero() {
            break;
        }
        let available = lot.qty_remaining_at(as_of);
        if available <= Decimal::ZERO {
            continue;
        }
        let take = available.min(remaining);
        allocations.push((lot.id, take));
        remaining -= take;
    }

    if remaining > Decimal::ZERO {
        let available_total: Decimal = candidates.iter().map(|l| l.qty_remaining_at(as_of)).sum();
        return Err(AppError::Validation(format!(
            "Cannot sell {quantity}; only {available_total} currently held at this broker"
        )));
    }

    Ok(allocations)
}

#[cfg(test)]
mod match_fifo_lots_tests {
    use super::*;

    fn lot(id: i64, date: &str, qty: &str, sells: Vec<(&str, &str)>) -> LotRecord {
        let acquisition_datetime = format!("{date}T00:00:00").parse::<NaiveDateTime>().unwrap();
        LotRecord {
            id,
            listing_id: 1,
            instrument_id: 1,
            isin: "IE00TEST0001".into(),
            name: "Test".into(),
            ticker: "TST".into(),
            exchange_mic: "XAMS".into(),
            instrument_type: InstrumentType::Etf,
            currency_code: "EUR".into(),
            qty_at_acquisition: qty.parse().unwrap(),
            price_per_unit: "10".parse().unwrap(),
            broker_id_at_acquisition: 1,
            transfers: vec![],
            existence_start: acquisition_datetime.date(),
            close_date: None,
            acquisition_datetime,
            acquisition_fx_rate: Decimal::ONE,
            fees: CurrencyPair::default(),
            sells: sells
                .into_iter()
                .map(|(d, q)| (d.parse().unwrap(), q.parse().unwrap()))
                .collect(),
        }
    }

    #[test]
    fn consumes_oldest_lot_first() {
        let older = lot(1, "2024-01-01", "10", vec![]);
        let newer = lot(2, "2024-06-01", "10", vec![]);
        // pass them in reverse to prove sorting, not insertion order, decides
        let candidates = [&newer, &older];
        let as_of = "2025-01-01".parse().unwrap();

        let allocations = match_fifo_lots(&candidates, "15".parse().unwrap(), as_of).unwrap();

        assert_eq!(
            allocations,
            vec![(1, "10".parse().unwrap()), (2, "5".parse().unwrap())]
        );
    }

    #[test]
    fn errors_when_quantity_exceeds_available() {
        let only = lot(1, "2024-01-01", "10", vec![]);
        let candidates = [&only];
        let as_of = "2025-01-01".parse().unwrap();

        let err = match_fifo_lots(&candidates, "11".parse().unwrap(), as_of).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn skips_a_lot_already_fully_sold() {
        let sold_out = lot(1, "2024-01-01", "10", vec![("2024-06-01", "10")]);
        let still_open = lot(2, "2024-03-01", "5", vec![]);
        let candidates = [&sold_out, &still_open];
        let as_of = "2025-01-01".parse().unwrap();

        let allocations = match_fifo_lots(&candidates, "5".parse().unwrap(), as_of).unwrap();

        assert_eq!(allocations, vec![(2, "5".parse().unwrap())]);
    }
}
