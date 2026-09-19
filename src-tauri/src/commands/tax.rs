use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;

use crate::{
    AppError,
    commands::{
        lot_data::{LotRecord, load_lot_records, match_fifo_lots},
        trade::{CreateSellTradeInput, MoneyInput},
    },
    parse_decimal_external, parse_decimal_internal,
};

#[derive(Debug, Clone, PartialEq, Eq, sqlx::Type, Serialize, Deserialize, specta::Type)]
#[sqlx(type_name = "TEXT", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Pre2026CostBasisMethod {
    Fotomoment,
    HistoricalElected,
    FlooredAtZero,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct Pre2026CostBasis {
    pub fotomoment_cost_eur: Decimal,
    pub historical_cost_eur: Decimal,
    pub method: Pre2026CostBasisMethod,
    pub tax_snapshot_2025_id: i64,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct AllocationTax {
    // fees excluded, EUR only, per Belgian CGT rules
    pub sale_price_eur: Decimal,
    pub taxable_buy_price_eur: Decimal,
    pub taxable_gain_eur: Decimal,
    pub sale_fx_rate_id: Option<i64>,
    pub buy_fx_rate_id: Option<i64>,
    /// Only Some for pre-2026 lots
    pub pre2026_cost_basis: Option<Pre2026CostBasis>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct AllocationComputation {
    pub origin_lot_id: i64,
    pub quantity: Decimal,
    /// Brussels-local calendar date.
    pub acquisition_date: NaiveDate,
    /// Fees included, EUR only.
    pub economic_gain_eur: Decimal,
    /// None when the instrument's `subject_to_cgt = false`.
    pub tax: Option<AllocationTax>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct SellComputation {
    pub isin: String,
    pub broker_id: i64,
    pub subject_to_cgt: bool,
    pub tax_year: Option<i64>,
    pub allocations: Vec<AllocationComputation>,
    pub total_economic_gain_eur: Decimal,
    pub total_taxable_gain_eur: Option<Decimal>,
}

pub fn to_brussels_date_from_utc(dt: DateTime<Utc>) -> NaiveDate {
    dt.with_timezone(&chrono_tz::Europe::Brussels).date_naive()
}

pub fn to_brussels_date(utc_naive: NaiveDateTime) -> NaiveDate {
    to_brussels_date_from_utc(DateTime::from_naive_utc_and_offset(utc_naive, Utc))
}

/// taxes only allow ECB rates
/// TODO: make universal function the get_rate command can also use
pub async fn resolve_ecb_rate_with_id(
    pool: &Pool<Sqlite>,
    currency: &str,
    date: NaiveDate,
) -> Result<Option<(i64, Decimal)>, AppError> {
    let row = sqlx::query!(
        r#"
        SELECT
            id as "id!",
            rate_to_eur
        FROM fx_rate
        WHERE currency = ?1
          AND source = 'ECB'
          AND date <= ?2
        ORDER BY date DESC
        LIMIT 1
        "#,
        currency,
        date,
    )
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => {
            let rate = parse_decimal_internal(&r.rate_to_eur, "fx_rate rate_to_eur")?;
            Ok(Some((r.id, rate)))
        }
        None => Ok(None),
    }
}

/// Circulaire 2026/C/74 art. 102 §4 (rn. 156-168)
/// `fotomoment_cost_eur` (F) is the DEFAULT cost basis: `allocated_qty ×
/// snap_price_per_unit_eur`, valid for lots acquired before 01.01.2026.
/// `historical_cost_eur` (H) is the ELECTIVE alternative: `allocated_qty ×
/// hist_cost_per_unit_eur`, available up to and including 31.12.2030,
/// and per rn. 167 can only ever reduce the gain toward zero, never create or deepen a loss.
/// returns (taxable_buy_price_eur, Pre2026CostBasisMethod)
pub fn apply_pre2026_formula(
    fotomoment_cost_eur: Decimal,
    historical_cost_eur: Decimal,
    sale_price_eur: Decimal,
    brussels_sale_date: NaiveDate,
) -> (Decimal, Pre2026CostBasisMethod) {
    let election_available =
        brussels_sale_date <= NaiveDate::from_ymd_opt(2030, 12, 31).expect("valid date");
    let capped_historical = historical_cost_eur.min(sale_price_eur);

    if election_available && capped_historical > fotomoment_cost_eur {
        if historical_cost_eur >= sale_price_eur {
            (sale_price_eur, Pre2026CostBasisMethod::FlooredAtZero)
        } else {
            (
                historical_cost_eur,
                Pre2026CostBasisMethod::HistoricalElected,
            )
        }
    } else {
        (fotomoment_cost_eur, Pre2026CostBasisMethod::Fotomoment)
    }
}

/// Only ever invoked for lots whose Brussels-local acquisition date is before 2026-01-01
/// Looks up this broker's own `tax_snapshot_2025` `(instrument_id, broker_id)`
/// no cross-broker aggregation (Circulaire 2026/C/74 rn. 144: FIFO is scoped per securities account).
/// Applies split-adjustment for corporate actions with `2025-12-31 < effective_date <= brussels_sale_date`,
/// INFO: because meerwaardebelasting does not care about the MIC, only the ISIN
/// same mechanism as `get_holdings`'s `split_multipliers` but keyed by `instrument_id` instead of `listing_id`
/// (ex. SPPW and SWRD are the same instument/ISIN but different listing/MIC)
pub async fn pre2026_cost_basis(
    pool: &Pool<Sqlite>,
    instrument_id: i64,
    broker_id: i64,
    allocated_qty: Decimal,
    sale_price_eur: Decimal,
    brussels_sale_date: NaiveDate,
) -> Result<Pre2026CostBasis, AppError> {
    let snapshot = sqlx::query!(
        r#"
        SELECT
            id as "id!",
            hist_cost_per_unit_eur,
            snap_price_per_unit_eur
        FROM
            tax_snapshot_2025
        WHERE 
            instrument_id = ?1
        AND
            broker_id = ?2
        "#,
        instrument_id,
        broker_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| {
        AppError::MissingData(format!(
            "Pre-2026 lot has no tax_snapshot_2025 row for instrument {instrument_id} at broker {broker_id}."
        ))
    })?;

    let mut hist_per_unit =
        parse_decimal_internal(&snapshot.hist_cost_per_unit_eur, "hist_cost_per_unit_eur")?;
    let mut snap_per_unit =
        parse_decimal_internal(&snapshot.snap_price_per_unit_eur, "snap_price_per_unit_eur")?;

    let cutoff_2025 = NaiveDate::from_ymd_opt(2025, 12, 31).expect("valid date");
    let splits = sqlx::query!(
        r#"
        SELECT
            ratio_from,
            ratio_to
        FROM
            corporate_action
        WHERE
            instrument_id = ?1
          AND
            action_type IN ('SPLIT', 'REVERSE_SPLIT')
          AND
            effective_date > ?2
          AND
            effective_date <= ?3
        ORDER BY
            effective_date ASC
        "#,
        instrument_id,
        cutoff_2025,
        brussels_sale_date
    )
    .fetch_all(pool)
    .await?;

    for split in splits {
        let from = parse_decimal_internal(&split.ratio_from, "ratio_from")?;
        let to = parse_decimal_internal(&split.ratio_to, "ratio_to")?;
        hist_per_unit = hist_per_unit * from / to;
        snap_per_unit = snap_per_unit * from / to;
    }

    let fotomoment_cost_eur = allocated_qty * snap_per_unit;
    let historical_cost_eur = allocated_qty * hist_per_unit;

    let (_, method) = apply_pre2026_formula(
        fotomoment_cost_eur,
        historical_cost_eur,
        sale_price_eur,
        brussels_sale_date,
    );

    Ok(Pre2026CostBasis {
        fotomoment_cost_eur,
        historical_cost_eur,
        method,
        tax_snapshot_2025_id: snapshot.id,
    })
}

/// Circulaire 2026/C/74, mainly art. 102 §4 and rn. 144
pub async fn compute_sell(
    pool: &Pool<Sqlite>,
    fields: &CreateSellTradeInput,
) -> Result<SellComputation, AppError> {
    // ---- validate ---------------------------------------------
    let quantity = parse_decimal_external(&fields.quantity, "quantity")?;
    if quantity <= Decimal::ZERO {
        return Err(AppError::Validation("quantity must be positive".into()));
    }
    let unit_price = parse_decimal_external(&fields.unit_price.amount, "unit_price")?;
    if unit_price <= Decimal::ZERO {
        return Err(AppError::Validation("unit_price must be positive".into()));
    }

    let parse_optional_fee =
        |raw: &Option<MoneyInput>, ctx: &'static str| -> Result<Option<Decimal>, AppError> {
            match raw {
                None => Ok(None),
                Some(v) if v.amount.is_empty() || v.amount == "0" => Ok(None),
                Some(v) => {
                    let d = parse_decimal_external(&v.amount, ctx)?;
                    if d < Decimal::ZERO {
                        return Err(AppError::Validation(format!("{ctx} must be non-negative")));
                    }
                    Ok(Some(d))
                }
            }
        };
    let broker_fee = parse_optional_fee(&fields.broker_fee.clone(), "broker_fee")?;
    let tob_fee = parse_optional_fee(&fields.tob_fee.clone(), "tob_fee")?;

    // ---- resolve listing ---------------------------------------
    let listing = sqlx::query!(
        r#"
        SELECT
            i.id              AS "instrument_id!",
            i.isin            AS "isin!",
            i.subject_to_cgt  AS "subject_to_cgt!",
            li.currency_code  AS "currency_code!"
        FROM listing li
        JOIN instrument i ON i.id = li.instrument_id
        WHERE li.id = ?1 AND li.delisted_at IS NULL
        "#,
        fields.listing_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| {
        AppError::NotFound(format!(
            "Listing {} not found (or delisted)",
            fields.listing_id
        ))
    })?;

    let isin = listing.isin.clone();
    let subject_to_cgt = listing.subject_to_cgt != 0;

    // ---- tax year + confirmed-year / cgt_parameters guards -----
    // Only relevant when subject_to_cgt is true
    let brussels_sale_date = to_brussels_date_from_utc(fields.executed_at);

    let tax_year: Option<i64> = if subject_to_cgt {
        let year = i64::from(brussels_sale_date.year());

        let already_confirmed = sqlx::query_scalar!(
            r#"
            SELECT
                is_confirmed
            FROM 
                cgt_exemption_usage
            WHERE
                tax_year = ?1
            "#,
            year
        )
        .fetch_optional(pool)
        .await?
        .map(|v| v != 0)
        .unwrap_or(false);

        if already_confirmed {
            return Err(AppError::Validation(format!(
                "Tax year {year} is already confirmed; amending a confirmed year is not supported"
            )));
        }

        let params_exist = sqlx::query!(
            r#"
            SELECT
                tax_year
            FROM
                cgt_parameters
            WHERE
                tax_year = ?1
            "#,
            year
        )
        .fetch_optional(pool)
        .await?
        .is_some();

        if !params_exist {
            return Err(AppError::Validation(format!(
                "No cgt_parameters configured for tax year {year}"
            )));
        }

        Some(year)
    } else {
        None
    };

    // ---- load broker-scoped candidate lots ----------------------
    let sale_date = fields.executed_at.date_naive();
    let all_lots = load_lot_records(pool).await?;
    let candidates: Vec<&LotRecord> = all_lots
        .iter()
        .filter(|l| {
            l.isin == isin
                && l.is_active_at(sale_date)
                && l.broker_id_as_of(sale_date) == fields.broker_id
        })
        .collect();

    // ---- chronology guard, broker-scoped -------------------------
    let latest_prior_sell: Option<NaiveDateTime> = sqlx::query_scalar!(
        r#"
        SELECT
            MAX(t.executed_at) AS "max_at: NaiveDateTime"
        FROM
            sell_allocation sa
        JOIN trade t ON t.id = sa.sell_trade_id
        JOIN lot l ON l.id = sa.origin_lot_id
        WHERE
            l.instrument_id = ?1
        AND
            t.broker_id = ?2
        "#,
        listing.instrument_id,
        fields.broker_id
    )
    .fetch_one(pool)
    .await?;

    if let Some(latest) = latest_prior_sell {
        if latest > fields.executed_at.naive_utc() {
            return Err(AppError::Validation(format!(
                "A later sell already exists for this instrument at this broker (on {latest}); backdating sales is not supported"
            )));
        }
    }

    // ---- FIFO match ----------------------------------------------
    let allocations = match_fifo_lots(&candidates, quantity, sale_date)?;
    let candidates_by_id: HashMap<i64, &LotRecord> =
        candidates.iter().map(|l| (l.id, *l)).collect();

    // ---- per-allocation economic + tax figures -------------------
    let (sale_fx_rate, sale_fx_rate_id) = if listing.currency_code == "EUR" {
        (Decimal::ONE, None)
    } else {
        let (id, rate) = resolve_ecb_rate_with_id(pool, &listing.currency_code, sale_date)
            .await?
            .ok_or_else(|| {
                AppError::MissingData(format!(
                    "No ECB rate for {} on or before {sale_date}",
                    listing.currency_code
                ))
            })?;
        (rate, Some(id))
    };

    let total_fees = broker_fee.unwrap_or(Decimal::ZERO) + tob_fee.unwrap_or(Decimal::ZERO);
    let cutoff_2026 = NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid date");

    let mut allocation_results = Vec::new();
    let mut total_economic_gain_eur = Decimal::ZERO;
    let mut total_taxable_gain_eur = if subject_to_cgt {
        Some(Decimal::ZERO)
    } else {
        None
    };

    for (lot_id, allocated_qty) in allocations {
        let lot = candidates_by_id[&lot_id];
        let acquisition_date = to_brussels_date(lot.acquisition_datetime);

        let allocation_fee_share = if quantity.is_zero() {
            Decimal::ZERO
        } else {
            (allocated_qty / quantity) * total_fees
        };

        let economic_gain_eur = (allocated_qty * unit_price * sale_fx_rate)
        // sell fees
            - allocation_fee_share
        // buy cost
            - lot.cost_contribution_eur(allocated_qty)
        // buy fees
            - lot.fees_eur_for_qty(allocated_qty);
        total_economic_gain_eur += economic_gain_eur;

        let tax = if subject_to_cgt {
            let sale_price_eur = allocated_qty * unit_price * sale_fx_rate;
            let lot_is_pre_2026 = acquisition_date < cutoff_2026;

            let (taxable_buy_price_eur, buy_fx_rate_id, pre2026_cost_basis) = if lot_is_pre_2026 {
                let basis = pre2026_cost_basis(
                    pool,
                    listing.instrument_id,
                    fields.broker_id,
                    allocated_qty,
                    sale_price_eur,
                    brussels_sale_date,
                )
                .await?;

                let price = match basis.method {
                    Pre2026CostBasisMethod::Fotomoment => basis.fotomoment_cost_eur,
                    Pre2026CostBasisMethod::HistoricalElected => basis.historical_cost_eur,
                    Pre2026CostBasisMethod::FlooredAtZero => sale_price_eur,
                };
                // no buy_fx_rate info, snapshot data is already EUR
                (price, None, Some(basis))
            } else {
                let buy_price_eur = lot.cost_contribution_eur(allocated_qty);
                let buy_fx_rate_id = if lot.currency_code == "EUR" {
                    None
                } else {
                    let (id, rate) = resolve_ecb_rate_with_id(
                        pool,
                        &lot.currency_code,
                        lot.acquisition_datetime.date(),
                    )
                    .await?
                    .ok_or_else(|| {
                        AppError::MissingData(format!(
                            "No ECB rate for {} on or before {} (lot {} acquisition)",
                            lot.currency_code,
                            lot.acquisition_datetime.date(),
                            lot.id
                        ))
                    })?;
                    debug_assert_eq!(
                        rate, lot.acquisition_fx_rate,
                        "acquisition FX rate mismatch for lot {}",
                        lot.id
                    );
                    Some(id)
                };
                // no pre 2026 basis as this is post 2025
                (buy_price_eur, buy_fx_rate_id, None)
            };

            let taxable_gain_eur = sale_price_eur - taxable_buy_price_eur;
            if let Some(total) = total_taxable_gain_eur.as_mut() {
                *total += taxable_gain_eur;
            }

            Some(AllocationTax {
                sale_price_eur,
                taxable_buy_price_eur,
                taxable_gain_eur,
                sale_fx_rate_id,
                buy_fx_rate_id,
                pre2026_cost_basis,
            })
        } else {
            None
        };

        allocation_results.push(AllocationComputation {
            origin_lot_id: lot_id,
            quantity: allocated_qty,
            acquisition_date,
            economic_gain_eur,
            tax,
        });
    }

    Ok(SellComputation {
        isin,
        broker_id: fields.broker_id,
        subject_to_cgt,
        tax_year,
        allocations: allocation_results,
        total_economic_gain_eur,
        total_taxable_gain_eur,
    })
}

#[cfg(test)]
mod pre2026_formula_tests {
    use super::*;

    fn d(s: &str) -> Decimal {
        s.parse().unwrap()
    }
    fn nd(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn rn167_voorbeeld1_floored_at_zero() {
        // bought 2013 @ 14.50, fotomoment(31.12.2025) @ 5, sold 07.12.2029 @ 13.50
        let hist = d("14.50");
        let foto = d("5");
        let sale = d("13.50");
        let (buy_price, method) = apply_pre2026_formula(foto, hist, sale, nd(2029, 12, 7));
        assert_eq!(buy_price, sale);
        assert_eq!(method, Pre2026CostBasisMethod::FlooredAtZero);
        // "geen belaste meerwaarde, maar ook geen minderwaarde"
        assert_eq!(sale - buy_price, d("0"));
    }

    #[test]
    fn rn167_voorbeeld2_fotomoment_past_election_cutoff() {
        // sold 07.12.2031, past the election cutoff
        let hist = d("14.50");
        let foto = d("5");
        let sale = d("13.50");
        let (buy_price, method) = apply_pre2026_formula(foto, hist, sale, nd(2031, 12, 7));
        assert_eq!(buy_price, foto);
        assert_eq!(method, Pre2026CostBasisMethod::Fotomoment);
        assert_eq!(sale - buy_price, d("8.50"));
    }

    #[test]
    fn rn167_voorbeeld3_historical_elected() {
        // aandelen A -> inbreng in B: fotomoment=21000, historical=22000, sale=22400
        let hist = d("22000");
        let foto = d("21000");
        let sale = d("22400");
        let (buy_price, method) = apply_pre2026_formula(foto, hist, sale, nd(2027, 1, 1));
        assert_eq!(buy_price, hist);
        assert_eq!(method, Pre2026CostBasisMethod::HistoricalElected);
        assert_eq!(sale - buy_price, d("400"));
    }

    #[test]
    fn rn176_voorbeeld1_fotomoment_minderwaarde() {
        // 100,000 units bought 03.04.2025 @ 1.38, fotomoment 2.05, sold late 2026 @ 1.95
        let hist = d("100000") * d("1.38");
        let foto = d("100000") * d("2.05");
        let sale = d("100000") * d("1.95");
        let (buy_price, method) = apply_pre2026_formula(foto, hist, sale, nd(2026, 12, 31));
        assert_eq!(buy_price, foto);
        assert_eq!(method, Pre2026CostBasisMethod::Fotomoment);
        assert_eq!(sale - buy_price, d("-10000"));
    }
}

#[cfg(test)]
mod sell_integration_tests {
    use super::*;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    async fn test_pool() -> Pool<Sqlite> {
        let options = SqliteConnectOptions::new()
            .filename(":memory:")
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .expect("in-memory sqlite pool");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("run migrations");
        pool
    }

    async fn base_fixture(pool: &Pool<Sqlite>, broker_id: i64, instrument_id: i64, tax_year: i64) {
        sqlx::query!("INSERT INTO currency (code) VALUES ('EUR') ON CONFLICT (code) DO NOTHING")
            .execute(pool)
            .await
            .unwrap();

        let broker_name = format!("Broker{broker_id}");
        sqlx::query!(
            "INSERT INTO broker (id, name) VALUES (?1, ?2)",
            broker_id,
            broker_name
        )
        .execute(pool)
        .await
        .unwrap();

        let isin = format!("IE00TEST{instrument_id:04}");
        sqlx::query!(
            r#"
            INSERT INTO instrument
                (id, isin, name, instrument_type, fsma_registered, accumulating, subject_to_cgt)
            VALUES (?1, ?2, 'Test Instrument', 'ETF', 0, 1, 1)
            "#,
            instrument_id,
            isin
        )
        .execute(pool)
        .await
        .unwrap();

        sqlx::query!(
            r#"
            INSERT INTO listing (id, instrument_id, exchange_mic, ticker, currency_code)
            VALUES (?1, ?2, 'XAMS', 'TST', 'EUR')
            "#,
            instrument_id,
            instrument_id
        )
        .execute(pool)
        .await
        .unwrap();

        sqlx::query!(
            r#"
            INSERT INTO cgt_parameters
                (tax_year, exemption_base_eur, exemption_cap_eur, carryforward_cap_eur)
            VALUES (?1, '10000', '15000', '1000')
            "#,
            tax_year
        )
        .execute(pool)
        .await
        .unwrap();

        sqlx::query!(
            r#"
            INSERT INTO cgt_exemption_usage (tax_year, carryforward_in_eur, is_confirmed)
            VALUES (?1, '0', 0)
            "#,
            tax_year
        )
        .execute(pool)
        .await
        .unwrap();
    }

    /// Inserts a BUY trade + its lot, both EUR-denominated.
    /// Returns the new lot id
    async fn insert_lot(
        pool: &Pool<Sqlite>,
        broker_id: i64,
        instrument_id: i64,
        listing_id: i64,
        executed_at: &str,
        qty: &str,
        price: &str,
        currency: &str,
    ) -> i64 {
        let executed_at: NaiveDateTime = executed_at
            .parse()
            .expect("valid datetime literal in test fixture");

        let trade_id = sqlx::query!(
            r#"
            INSERT INTO trade
                (listing_id, broker_id, side, quantity, price, executed_at)
            VALUES (?1, ?2, 'BUY', ?3, ?4, ?5)
            "#,
            listing_id,
            broker_id,
            qty,
            price,
            executed_at
        )
        .execute(pool)
        .await
        .unwrap()
        .last_insert_rowid();

        sqlx::query!(
            r#"
            INSERT INTO lot
                (broker_id_at_acquisition, instrument_id, listing_id,
                 source_trade_id, qty_at_acquisition, price_currency_code, price_per_unit)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            broker_id,
            instrument_id,
            listing_id,
            trade_id,
            qty,
            currency,
            price
        )
        .execute(pool)
        .await
        .unwrap()
        .last_insert_rowid()
    }

    async fn insert_tax_snapshot(
        pool: &Pool<Sqlite>,
        instrument_id: i64,
        broker_id: i64,
        qty: &str,
        hist_cost: &str,
        snap_price: &str,
    ) {
        sqlx::query!(
            r#"
            INSERT INTO tax_snapshot_2025
                (instrument_id, broker_id, qty_at_snapshot, hist_cost_per_unit_eur, snap_price_per_unit_eur)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
            instrument_id,
            broker_id,
            qty,
            hist_cost,
            snap_price
        )
        .execute(pool)
        .await
        .unwrap();
    }

    fn d(s: &str) -> Decimal {
        s.parse().unwrap()
    }

    /// A sell that FIFO-matches against both a pre-2026 lot and a post-2025 lot in the same call
    #[tokio::test]
    async fn mixed_era_sell_computes_both_branches_independently() {
        let pool = test_pool().await;
        let broker_id = 1;
        let instrument_id = 1;
        let listing_id = 1;
        base_fixture(&pool, broker_id, instrument_id, 2026).await;

        // Pre-2026 lot: 10 units bought 2024-06-01 @ 25.00 EUR.
        insert_lot(
            &pool,
            broker_id,
            instrument_id,
            listing_id,
            "2024-06-01T10:00:00",
            "10",
            "25.00",
            "EUR",
        )
        .await;

        insert_tax_snapshot(&pool, instrument_id, broker_id, "10", "28.00", "22.00").await;

        // Post-2025 lot: 5 units bought 2026-03-01 @ 40.00 EUR.
        insert_lot(
            &pool,
            broker_id,
            instrument_id,
            listing_id,
            "2026-03-01T10:00:00",
            "5",
            "40.00",
            "EUR",
        )
        .await;

        let fields = CreateSellTradeInput {
            listing_id,
            broker_id,
            quantity: "12".into(),
            unit_price: MoneyInput {
                amount: "50.00".into(),
                currency: "EUR".into(),
            },
            executed_at: "2026-09-01T10:00:00Z".parse().unwrap(),
            broker_fee: None,
            tob_fee: None,
        };

        let computation = compute_sell(&pool, &fields).await.unwrap();

        assert_eq!(computation.allocations.len(), 2);

        // FIFO takes the older (pre-2026) lot first: all 10 units, plus 2
        // of the 5 available post-2025 units to reach the requested 12.
        let pre2026_alloc = &computation.allocations[0];
        assert_eq!(pre2026_alloc.quantity, d("10"));

        let pre2026_tax = pre2026_alloc.tax.as_ref().expect("subject_to_cgt is true");
        let basis = pre2026_tax
            .pre2026_cost_basis
            .as_ref()
            .expect("pre-2026 lot must carry a cost basis breakdown");

        // F = 10 * 22.00 = 220; H = 10 * 28.00 = 280.
        // Election helps here (F < H < S), so HistoricalElected should win.
        assert_eq!(basis.method, Pre2026CostBasisMethod::HistoricalElected);
        assert_eq!(pre2026_tax.sale_price_eur, d("500"));
        assert_eq!(pre2026_tax.taxable_buy_price_eur, d("280"));
        assert_eq!(pre2026_tax.taxable_gain_eur, d("220"));

        let post2025_alloc = &computation.allocations[1];
        assert_eq!(post2025_alloc.quantity, d("2"));
        let post2025_tax = post2025_alloc.tax.as_ref().expect("subject_to_cgt is true");
        assert!(post2025_tax.pre2026_cost_basis.is_none());

        assert_eq!(post2025_tax.sale_price_eur, d("100"));
        assert_eq!(post2025_tax.taxable_buy_price_eur, d("80"));
        assert_eq!(post2025_tax.taxable_gain_eur, d("20"));

        assert_eq!(computation.total_economic_gain_eur, d("270"));
        assert_eq!(computation.total_taxable_gain_eur, Some(d("240")));
        assert_eq!(computation.tax_year, Some(2026));
    }

    /// Two brokers hold the same instrument.
    /// Broker B's lot is chronologically newer than broker A's
    /// if FIFO were (incorrectly) scoped globally per instrument instead of per broker (Circulaire 2026/C/74 rn. 144),
    /// a sell at broker B would incorrectly reach back into broker A's older, pre-2026 lot.
    #[tokio::test]
    async fn fifo_is_scoped_per_broker_not_globally_per_instrument() {
        let pool = test_pool().await;
        let instrument_id = 1;
        let listing_id = 1;
        let broker_a_id = 1;
        let broker_b_id = 2;
        base_fixture(&pool, broker_a_id, instrument_id, 2026).await;
        sqlx::query!(
            r#"
            INSERT INTO
                broker (id, name)
            VALUES
                (?1, 'BrokerB')
            "#,
            broker_b_id
        )
        .execute(&pool)
        .await
        .unwrap();

        // Broker A: 5 units bought 2025-06-01 — pre-2026, chronologically OLDER.
        insert_lot(
            &pool,
            broker_a_id,
            instrument_id,
            listing_id,
            "2025-06-01T10:00:00",
            "5",
            "20.00",
            "EUR",
        )
        .await;
        insert_tax_snapshot(&pool, instrument_id, broker_a_id, "5", "20.00", "24.00").await;

        // Broker B: 5 units bought 2026-01-15 — post-2025, chronologically NEWER.
        insert_lot(
            &pool,
            broker_b_id,
            instrument_id,
            listing_id,
            "2026-01-15T10:00:00",
            "5",
            "30.00",
            "EUR",
        )
        .await;

        let fields = CreateSellTradeInput {
            listing_id,
            broker_id: broker_a_id,
            quantity: "5".into(),
            unit_price: MoneyInput {
                amount: "50.00".into(),
                currency: "EUR".into(),
            },
            executed_at: "2026-09-01T10:00:00Z".parse().unwrap(),
            broker_fee: None,
            tob_fee: None,
        };

        // Selling all 5 shares at broker A must succeed using only broker A's lot.
        let computation_a = compute_sell(&pool, &fields).await.unwrap();
        assert_eq!(computation_a.allocations.len(), 1);
        assert_eq!(computation_a.allocations[0].quantity, d("5"));
        assert!(
            computation_a.allocations[0]
                .tax
                .as_ref()
                .unwrap()
                .pre2026_cost_basis
                .is_some(),
            "broker A's only lot is pre-2026"
        );

        // A 1-share sell at broker B must use broker B's post-2025 lot, not
        // broker A's chronologically-older pre-2026 one.
        let fields_b = CreateSellTradeInput {
            quantity: "1".into(),
            broker_id: broker_b_id,
            ..fields
        };
        let computation_b = compute_sell(&pool, &fields_b).await.unwrap();
        assert_eq!(computation_b.allocations.len(), 1);
        assert!(
            computation_b.allocations[0]
                .tax
                .as_ref()
                .unwrap()
                .pre2026_cost_basis
                .is_none(),
            "broker B's only lot is post-2025 and must not fall back to broker A's"
        );
    }
}
