use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

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
