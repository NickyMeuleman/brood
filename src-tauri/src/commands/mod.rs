pub mod holdings;

use crate::{parse_decimal, AppError};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::{Pool, Sqlite};
use std::ops::AddAssign;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, Default)]
pub struct Metrics {
    pub cost: Decimal,
    pub gain: Decimal,
    pub pct_gain: Decimal,
    pub fees: Decimal,
    pub pct_fees: Decimal,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, Default)]
pub struct NetGross {
    pub gross: Metrics,
    pub net: Metrics,
}

impl NetGross {
    pub fn calculate(
        start_value: Decimal,
        end_value: Decimal,
        gross_cost: Decimal,
        fees: Decimal,
    ) -> Self {
        let gross_gain = end_value - start_value - gross_cost;
        let gross_base = start_value + gross_cost;
        let gross_pct_gain = if gross_base.is_zero() {
            Decimal::ZERO
        } else {
            gross_gain / gross_base
        };

        let net_cost = gross_cost + fees;
        let net_gain = gross_gain - fees;
        let net_base = start_value + net_cost;
        let net_pct_gain = if net_base.is_zero() {
            Decimal::ZERO
        } else {
            net_gain / net_base
        };

        let pct_fees = if gross_cost.is_zero() {
            Decimal::ZERO
        } else {
            fees / gross_cost
        };

        Self {
            gross: Metrics {
                cost: gross_cost,
                gain: gross_gain,
                pct_gain: gross_pct_gain,
                fees,
                pct_fees,
            },
            net: Metrics {
                cost: net_cost,
                gain: net_gain,
                pct_gain: net_pct_gain,
                fees,
                pct_fees,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, Default)]
pub struct CurrencyPair {
    pub local: NetGross,
    pub eur: NetGross,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
pub struct PeriodContext {
    pub start_quantity: Decimal,
    pub start_unit_price: Option<Decimal>,
    pub start_unit_price_eur: Option<Decimal>,
    pub start_market_value: Decimal,
    pub start_market_value_eur: Decimal,
    pub metrics: CurrencyPair,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub enum Period {
    FiveDays,
    OneMonth,
    SixMonths,
    OneYear,
    FiveYears,
    Ytd,
    AllTime,
}

pub async fn get_rate(
    pool: &Pool<Sqlite>,
    currency_code: &str,
    date: NaiveDate,
) -> Result<Decimal, AppError> {
    if currency_code == "EUR" {
        return Ok(Decimal::ONE);
    }

    let row = sqlx::query!(
        r#"
        SELECT rate_to_eur
        FROM fx_rate
        WHERE currency = ?1
          AND date <= ?2
        ORDER BY date DESC
        LIMIT 1
        "#,
        currency_code,
        date,
    )
    .fetch_optional(pool)
    .await
    .map_err(AppError::from)?;

    let rate = row
        .ok_or_else(|| {
            AppError::Database(format!(
                "No FX rate for {currency_code} on or before {date}"
            ))
        })?
        .rate_to_eur;

    parse_decimal(&rate, "FX rate")
}

impl AddAssign for Metrics {
    fn add_assign(&mut self, other: Self) {
        self.cost += other.cost;
        self.gain += other.gain;
        self.fees += other.fees;
        // NOTE: don't add pct_gain or pct_fees here because percentages can't be summed.
    }
}

impl AddAssign for NetGross {
    fn add_assign(&mut self, other: Self) {
        self.gross += other.gross;
        self.net += other.net;
    }
}
