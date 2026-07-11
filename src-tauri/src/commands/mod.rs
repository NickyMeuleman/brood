pub mod chart;
pub mod holdings;
pub mod lot_data;
pub mod sync;
pub mod trade;

use crate::{AppError, db::Db, parse_decimal_internal};
use chrono::{DateTime, Datelike, Days, Months, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::{Pool, Sqlite};
use std::{
    collections::{BTreeMap, HashMap},
    ops::AddAssign,
};
use tauri::State;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
pub struct Metrics {
    pub cost: Decimal,
    pub gain: Option<Decimal>,
    pub pct_gain: Option<Decimal>,
}
/// Manual implementation to start options at Some(0) instead of None
impl Default for Metrics {
    fn default() -> Self {
        Self {
            cost: Decimal::ZERO,
            gain: Some(Decimal::ZERO),
            pct_gain: Some(Decimal::ZERO),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, Default)]
pub struct NetGross<T> {
    pub gross: T,
    pub net: T,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, Default)]
pub struct CurrencyPair<T> {
    pub local: T,
    pub eur: T,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, Default)]
pub struct Performance {
    pub gross: Metrics,
    pub net: Metrics,
    pub fees: Decimal,
    // Fees as a fraction of acquisition cost (total_fees_eur / total_cost_eur).
    // fees and cost share the same executed_at date,
    // so the FX rate cancels out and both formulations produce identical results.
    /// fee drag, % of gross cost that needed to be added
    pub fee_drag: Decimal,
}

impl Performance {
    pub fn recalc_pcts(&mut self, start_value: Option<Decimal>) {
        let gross_base = start_value.map(|n| n + self.gross.cost);
        let net_base = start_value.map(|n| n + self.net.cost);

        let get_pct_gain = |gain: Option<Decimal>, base: Option<Decimal>| match (gain, base) {
            (Some(gain), Some(base)) if !base.is_zero() => Some(gain / base),
            _ => None,
        };

        self.gross.pct_gain = get_pct_gain(self.gross.gain, gross_base);
        self.net.pct_gain = get_pct_gain(self.net.gain, net_base);
        self.fee_drag = if self.gross.cost.is_zero() {
            Decimal::ZERO
        } else {
            self.fees / self.gross.cost
        };
    }

    pub fn calculate(
        start_value: Option<Decimal>,
        end_value: Decimal,
        gross_cost: Decimal,
        fees: Decimal,
    ) -> Self {
        let gross_gain = start_value.map(|n| end_value - n - gross_cost);
        let gross_base = start_value.map(|n| n + gross_cost);

        let get_pct_gain = |gain: Option<Decimal>, base: Option<Decimal>| match (gain, base) {
            (Some(gain), Some(base)) if !base.is_zero() => Some(gain / base),
            _ => None,
        };

        let gross_pct_gain = get_pct_gain(gross_gain, gross_base);

        let net_cost = gross_cost + fees;
        let net_gain = gross_gain.map(|n| n - fees);
        let net_base = start_value.map(|n| n + net_cost);
        let net_pct_gain = get_pct_gain(net_gain, net_base);

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
            },
            net: Metrics {
                cost: net_cost,
                gain: net_gain,
                pct_gain: net_pct_gain,
            },
            fees,
            fee_drag: pct_fees,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
pub struct Snapshot {
    pub quantity: Decimal,
    pub unit_price: CurrencyPair<Decimal>,
    pub value: CurrencyPair<Decimal>,
    pub unit_price_basis: CurrencyPair<NetGross<Decimal>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, Default)]
pub struct PeriodContext {
    pub start_quantity: Decimal,
    pub start_unit_price: Option<CurrencyPair<Decimal>>,
    pub start_value: Option<CurrencyPair<Decimal>>,
    pub perf: CurrencyPair<Performance>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub enum Period {
    FiveDays,
    OneMonth,
    SixMonths,
    OneYear,
    FiveYears,
    Ytd,
    AllTime,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, Default)]
pub struct DayTotals {
    pub value_eur: Decimal,
    pub invested_eur: NetGross<Decimal>, // gross = no fees, net = with fees
}

pub fn latest_on_or_before<V: Copy>(map: &BTreeMap<NaiveDate, V>, date: NaiveDate) -> Option<V> {
    map.range(..=date).next_back().map(|(_, &v)| v)
}

impl AddAssign for Metrics {
    fn add_assign(&mut self, other: Self) {
        self.cost += other.cost;
        self.gain = self.gain.zip(other.gain).map(|(a, b)| a + b);
        // NOTE: don't add pct_gain here because percentages can't be summed.
    }
}

impl AddAssign for Performance {
    fn add_assign(&mut self, other: Self) {
        self.gross += other.gross;
        self.net += other.net;
        self.fees += other.fees;
        // NOTE: don't add pct_fees here because percentages can't be summed.
    }
}

pub fn period_start(today: NaiveDate, period: Period) -> Option<NaiveDate> {
    match period {
        Period::AllTime => None,
        Period::FiveDays => today.checked_sub_days(Days::new(5)),
        Period::OneMonth => today.checked_sub_months(Months::new(1)),
        Period::SixMonths => today.checked_sub_months(Months::new(6)),
        Period::OneYear => today.checked_sub_months(Months::new(12)),
        Period::FiveYears => today.checked_sub_months(Months::new(60)),
        Period::Ytd => NaiveDate::from_ymd_opt(today.year(), 1, 1),
    }
}

pub async fn get_rates(
    pool: &Pool<Sqlite>,
) -> Result<HashMap<String, BTreeMap<NaiveDate, Decimal>>, AppError> {
    sqlx::query!(
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
    .await?
    .into_iter()
    .try_fold(
        HashMap::<String, BTreeMap<NaiveDate, Decimal>>::new(),
        |mut acc, row| {
            acc.entry(row.currency).or_default().insert(
                row.date,
                parse_decimal_internal(&row.rate_to_eur, "fx_rate rate_to_eur")?,
            );
            Ok(acc)
        },
    )
}

pub async fn get_prices(
    pool: &Pool<Sqlite>,
    today: NaiveDate,
) -> Result<HashMap<i64, BTreeMap<NaiveDate, Decimal>>, AppError> {
    sqlx::query!(
        r#"
        SELECT
            listing_id,
            date AS "date!: NaiveDate",
            close
        FROM price_history
        WHERE date <= ?1
        ORDER BY listing_id, date
        "#,
        today
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .try_fold(
        HashMap::<i64, BTreeMap<NaiveDate, Decimal>>::new(),
        |mut acc, row| {
            acc.entry(row.listing_id)
                .or_default()
                .insert(row.date, parse_decimal_internal(&row.close, "close price")?);
            Ok(acc)
        },
    )
}

pub async fn get_prices_on_or_before(
    pool: &Pool<Sqlite>,
    date: NaiveDate,
) -> Result<HashMap<i64, Decimal>, AppError> {
    // listing_id -> latest close
    sqlx::query!(
        r#"
        SELECT
            listing_id,
            close
        FROM (
            SELECT 
                listing_id, 
                close, 
                ROW_NUMBER() OVER (PARTITION BY listing_id ORDER BY date DESC) as rn
            FROM price_history
            WHERE date <= ?1
        )
        WHERE rn = 1
        "#,
        date
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| {
        Ok((
            r.listing_id,
            parse_decimal_internal(&r.close, "price_history close")?,
        ))
    })
    .collect()
}

pub async fn get_rates_on_or_before(
    pool: &Pool<Sqlite>,
    date: NaiveDate,
) -> Result<HashMap<String, Decimal>, AppError> {
    sqlx::query!(
        r#"
        SELECT
            currency,
            rate_to_eur
        FROM (
            SELECT
                currency,
                rate_to_eur,
                ROW_NUMBER() OVER (PARTITION BY currency ORDER BY date DESC) as rn
            FROM fx_rate
            WHERE date <= ?1
        )
        WHERE rn = 1
        "#,
        date
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| {
        Ok((
            r.currency,
            parse_decimal_internal(&r.rate_to_eur, "fx_rate rate_to_eur")?,
        ))
    })
    .collect()
}

pub fn resolve_rate(
    currency: &str,
    rates: &HashMap<String, Decimal>,
    context: &str,
) -> Result<Decimal, AppError> {
    if currency == "EUR" {
        return Ok(Decimal::ONE);
    }
    rates
        .get(currency)
        .copied()
        .ok_or_else(|| AppError::MissingData(format!("No FX rate for {currency}: {context}")))
}

#[tauri::command]
#[specta::specta]
pub async fn get_rate(
    db: State<'_, Db>,
    currency_code: &str,
    date: DateTime<Utc>,
) -> Result<Decimal, AppError> {
    if currency_code == "EUR" {
        return Ok(Decimal::ONE);
    }
    let date = date.naive_utc().date();

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
    .fetch_optional(&db.pool)
    .await
    .map_err(AppError::from)?;

    let rate = row
        .ok_or_else(|| {
            AppError::MissingData(format!(
                "No FX rate for {currency_code} on or before {date}"
            ))
        })?
        .rate_to_eur;

    parse_decimal_internal(&rate, "FX rate")
}

#[tauri::command]
#[specta::specta]
pub async fn get_price(
    db: State<'_, Db>,
    listing_id: i64,
    date: DateTime<Utc>,
) -> Result<Decimal, AppError> {
    let date = date.naive_utc().date();

    let row = sqlx::query!(
        r#"
        SELECT close
        FROM price_history
        WHERE listing_id = ?1
          AND date <= ?2
        ORDER BY date DESC
        LIMIT 1
        "#,
        listing_id,
        date,
    )
    .fetch_optional(&db.pool)
    .await
    .map_err(AppError::from)?;

    let close = row
        .ok_or_else(|| {
            AppError::MissingData(format!(
                "No price for listing {listing_id} on or before {date}"
            ))
        })?
        .close;

    parse_decimal_internal(&close, "price_history close")
}
