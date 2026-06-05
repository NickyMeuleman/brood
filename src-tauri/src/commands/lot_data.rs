use crate::commands::latest_on_or_before;
use crate::commands::CurrencyPair;
use crate::db::types::InstrumentType;
use crate::parse_decimal;
use crate::AppError;
use chrono::{NaiveDate, NaiveDateTime};
use rust_decimal::Decimal;
use sqlx::{Pool, Sqlite};
use std::collections::{BTreeMap, HashMap};

/// Everything known about a lot that is independent of any specific date.
#[derive(Debug, Clone)]
pub struct LotRecord {
    // --- identity (used by get_holdings for table rows) ---
    pub id: i64,
    pub listing_id: i64,
    pub isin: String,
    pub name: String,
    pub ticker: String,
    pub exchange_mic: String,
    pub instrument_type: InstrumentType,
    pub currency_code: String,

    // --- lot facts ---
    pub qty_at_acquisition: Decimal,
    pub price_per_unit: Decimal, // in listing currency

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
    pub acquisition_cost_date: NaiveDate,
    /// EUR/listing_currency rate on `acquisition_cost_date`. Already resolved
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
}

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
    // ---- 1. All lots -------------------------------------------------------
    // ORDER BY l.id ASC is vital for handling child CA lots topologically!
    let raw_lots = sqlx::query!(
        r#"
        SELECT
            l.id AS lot_id,
            l.listing_id,
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

    // ---- 2. lot_close dates ------------------------------------------------
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

    // ---- 3. CA effective dates (existence_start for CA-originated lots) -----
    // todo? handle CA lots that originate from older CA lots? recursive?
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

    // ---- 4. Full FX rate history -------------------------------------------
    // Loaded in full so that fee computation (step 5) and acquisition FX rate
    // resolution (step 8) can both use latest_on_or_before without further
    // database round-trips.
    let mut all_rates: HashMap<String, BTreeMap<NaiveDate, Decimal>> = HashMap::new();
    let fx_rows = sqlx::query!(
        r#"
        SELECT
            date AS "date!: NaiveDate",
            currency,
            rate_to_eur
        FROM fx_rate
        ORDER BY date
        "#
    )
    .fetch_all(pool)
    .await?;

    for row in fx_rows {
        let rate = parse_decimal(&row.rate_to_eur, "fx_rate rate_to_eur")?;
        all_rates
            .entry(row.currency)
            .or_default()
            .insert(row.date, rate);
    }

    // ---- 5. Trade fees -----------------------------------------------------
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
        let fee = parse_decimal(&r.fee_amount, "fee amount")?;
        let trade_qty = parse_decimal(&r.trade_qty, "trade qty")?;
        let date = r.executed_at.date();

        let fee_eur = if r.fee_currency == "EUR" {
            fee
        } else {
            let rate = all_rates
                .get(&r.fee_currency)
                .and_then(|m| latest_on_or_before(m, date))
                .ok_or_else(|| {
                    AppError::Database(format!(
                        "No FX rate for {} on {date} (fee currency)",
                        r.fee_currency
                    ))
                })?;
            fee * rate
        };

        // Express fee in listing currency for local performance calculations.
        // question: some stocks have mixed currency fees, eg broker fees in eur and stamp duty in
        // pounds
        let fee_local = if r.fee_currency == r.listing_currency {
            fee
        } else if r.listing_currency == "EUR" {
            fee_eur
        } else {
            let listing_rate = all_rates
                .get(&r.listing_currency)
                .and_then(|m| latest_on_or_before(m, date))
                .ok_or_else(|| {
                    AppError::Database(format!(
                        "No FX rate for {} on {date} (listing currency)",
                        r.listing_currency
                    ))
                })?;
            fee_eur / listing_rate
        };

        let entry = fee_by_trade.entry(r.trade_id).or_insert(TradeFeeAgg {
            trade_qty,
            fees: CurrencyPair::default(),
        });
        // trade_qty is identical for every fee row of the same trade; overwriting is idempotent.
        entry.trade_qty = trade_qty;
        entry.fees.local += fee_local;
        entry.fees.eur += fee_eur;
    }

    // ---- 6. Sell allocations with dates ------------------------------------
    let mut sells_by_lot: HashMap<i64, Vec<(NaiveDate, Decimal)>> = HashMap::new();
    // question: why DATE() and not the same pattern ar previously where you query the real
    // type and call .date() in a processing loop?
    let sell_rows = sqlx::query!(
        r#"
        SELECT
            sa.origin_lot_id,
            sa.quantity,
            DATE(t.executed_at) AS "sell_date!: NaiveDate"
        FROM sell_allocation sa
        JOIN trade t ON t.id = sa.sell_trade_id
        ORDER BY sa.origin_lot_id, t.executed_at
        "#
    )
    .fetch_all(pool)
    .await?;

    for row in sell_rows {
        let qty = parse_decimal(&row.quantity, "sell quantity")?;
        sells_by_lot
            .entry(row.origin_lot_id)
            .or_default()
            .push((row.sell_date, qty));
    }

    // ---- 7. Traverse lots in ascending ID order ----------------------------
    //
    // Parent lots always have lower IDs than their CA children, so ascending
    // ID order guarantees every parent is resolved before its children.
    //
    // For each lot we compute:
    //   acquisition_cost_date — the original trade date, propagated upward
    //   fees                  — attributed portion of trade fees, split by cost ratio
    let mut acquisition_cost_dates: HashMap<i64, NaiveDate> = HashMap::new();
    let mut lot_fees: HashMap<i64, CurrencyPair<Decimal>> = HashMap::new();
    let mut lot_costs: HashMap<i64, Decimal> = HashMap::new();

    for r in &raw_lots {
        let qty = parse_decimal(&r.qty_at_acquisition, "lot qty")?;
        let price = parse_decimal(&r.price_per_unit, "lot price")?;
        let cost = qty * price;
        lot_costs.insert(r.lot_id, cost);

        let (acq_date, fees) = match (r.source_trade_id, r.parent_lot_id) {
            // Trade-originated lot: use the trade's execution date directly.
            (Some(trade_id), _) => {
                let date = r
                    .trade_executed_at
                    .ok_or_else(|| {
                        AppError::Database(format!(
                            "Lot {} has source_trade_id but missing trade executed_at",
                            r.lot_id
                        ))
                    })?
                    .date();

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

                (date, fees)
            }

            // CA-originated lot: inherit date and fees from the parent,
            // proportioned by the ratio of this lot's cost to the parent's cost.
            (None, Some(parent_id)) => {
                let parent_acq = *acquisition_cost_dates.get(&parent_id).ok_or_else(|| {
                    AppError::Database(format!(
                        "Parent lot {parent_id} not yet processed before child lot {}",
                        r.lot_id
                    ))
                })?;
                let parent_cost = *lot_costs.get(&parent_id).ok_or_else(|| {
                    AppError::Database(format!("Cost missing for parent lot {parent_id}"))
                })?;
                let parent_fees = *lot_fees.get(&parent_id).ok_or_else(|| {
                    AppError::Database(format!("Fees missing for parent lot {parent_id}"))
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

                (parent_acq, fees)
            }

            (None, None) => {
                return Err(AppError::Database(format!(
                    "Lot {} has neither source_trade_id nor parent_lot_id",
                    r.lot_id
                )))
            }
        };

        acquisition_cost_dates.insert(r.lot_id, acq_date);
        lot_fees.insert(r.lot_id, fees);
    }

    // ---- 8. Build LotRecords -----------------------------------------------
    let mut records = Vec::with_capacity(raw_lots.len());

    for r in &raw_lots {
        let acq_cost_date = acquisition_cost_dates[&r.lot_id];

        let acquisition_fx_rate = if r.currency_code == "EUR" {
            Decimal::ONE
        } else {
            all_rates
                .get(&r.currency_code)
                .and_then(|m| latest_on_or_before(m, acq_cost_date))
                .ok_or_else(|| {
                    AppError::Database(format!(
                        "No FX rate for {} on {acq_cost_date} (lot {} acquisition)",
                        r.currency_code, r.lot_id
                    ))
                })?
        };

        // Trade lots exist from their trade execution date.
        // CA lots exist from the corporate action's effective date.
        let existence_start = if r.source_trade_id.is_some() {
            acq_cost_date
        } else {
            *ca_start_dates.get(&r.lot_id).ok_or_else(|| {
                AppError::Database(format!(
                    "CA lot {} has no corporate_action effective_date",
                    r.lot_id
                ))
            })?
        };

        records.push(LotRecord {
            id: r.lot_id,
            listing_id: r.listing_id,
            isin: r.isin.clone(),
            name: r.name.clone(),
            ticker: r.ticker.clone(),
            exchange_mic: r.exchange_mic.clone(),
            instrument_type: r.instrument_type.clone(),
            currency_code: r.currency_code.clone(),
            qty_at_acquisition: parse_decimal(&r.qty_at_acquisition, "qty_at_acquisition")?,
            price_per_unit: parse_decimal(&r.price_per_unit, "price_per_unit")?,
            existence_start,
            close_date: lot_close_dates.get(&r.lot_id).copied(),
            acquisition_cost_date: acq_cost_date,
            acquisition_fx_rate,
            fees: lot_fees[&r.lot_id],
            // remove() is fine here: raw_lots is borrowed immutably,
            // sells_by_lot is a separate HashMap, and lot IDs are unique.
            sells: sells_by_lot.remove(&r.lot_id).unwrap_or_default(),
        });
    }

    Ok(records)
}
