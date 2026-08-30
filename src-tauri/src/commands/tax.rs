use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};

use crate::{AppError, parse_decimal_internal};

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
    pub buy_price_eur: Decimal,
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
/// returns (buy_price_eur, Pre2026CostBasisMethod)
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
/// WARN: because meerwaardebelasting does not care about the MIC, only the ISIN
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
