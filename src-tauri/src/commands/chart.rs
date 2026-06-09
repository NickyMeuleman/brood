use crate::commands::lot_data::load_lot_records;
use crate::commands::{latest_on_or_before, period_start, DayTotals, NetGross, Period};
use crate::{db::Db, parse_decimal, AppError};
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::Serialize;
use specta::Type;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use tauri::State;

#[derive(Debug, Serialize, Type)]
pub struct PortfolioDataPoint {
    pub date: NaiveDate,
    pub totals: DayTotals,
}

#[derive(Debug, Serialize, Type)]
pub struct PortfolioHistory {
    pub points: Vec<PortfolioDataPoint>,
}

#[tauri::command]
#[specta::specta]
pub async fn get_portfolio_history(
    db: State<'_, Db>,
    period: Period,
) -> Result<PortfolioHistory, AppError> {
    let today = Utc::now().date_naive();
    let period_start = period_start(today, period);
    let lot_records = load_lot_records(&db.pool).await?;

    if lot_records.is_empty() {
        return Ok(PortfolioHistory { points: Vec::new() });
    }

    // For AllTime, use the earliest existence_start across all lots.
    let start_date = period_start.unwrap_or_else(|| {
        lot_records
            .iter()
            .map(|l| l.existence_start)
            .min()
            .unwrap_or(today)
    });

    // Load the full price history up to today (no start_date filter).
    // This ensures latest_on_or_before always finds the correct price even
    // when a listing has no data on the exact first day of the range.
    let mut prices: HashMap<i64, BTreeMap<NaiveDate, Decimal>> = HashMap::new();
    let price_rows = sqlx::query!(
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
    .fetch_all(&db.pool)
    .await?;

    for row in price_rows {
        let close = parse_decimal(&row.close, "close price")?;
        prices
            .entry(row.listing_id)
            .or_default()
            .insert(row.date, close);
    }

    // Load the full FX rate history up to today for the same reason.
    let mut rates: HashMap<String, BTreeMap<NaiveDate, Decimal>> = HashMap::new();
    let fx_rows = sqlx::query!(
        r#"
        SELECT
            currency,
            date AS "date!: NaiveDate",
            rate_to_eur
        FROM fx_rate
        WHERE date <= ?1
        ORDER BY currency, date
        "#,
        today
    )
    .fetch_all(&db.pool)
    .await?;

    for row in fx_rows {
        let rate = parse_decimal(&row.rate_to_eur, "rate_to_eur")?;
        rates
            .entry(row.currency)
            .or_default()
            .insert(row.date, rate);
    }

    // Only consider dates where at least one held listing has price data.
    let active_listing_ids: HashSet<i64> = lot_records.iter().map(|l| l.listing_id).collect();

    let dates: BTreeSet<NaiveDate> = prices
        .iter()
        .filter(|(id, _)| active_listing_ids.contains(id))
        .flat_map(|(_, date_map)| date_map.keys().copied())
        .filter(|&d| d >= start_date && d <= today)
        .collect();

    let mut points = Vec::with_capacity(dates.len());

    for date in dates {
        let mut value_eur = Decimal::ZERO;
        let mut cost_gross = Decimal::ZERO;
        let mut cost_net = Decimal::ZERO;

        for lot in &lot_records {
            if !lot.is_active_at(date) {
                continue;
            }

            let qty = lot.qty_remaining_at(date);

            let price = match prices
                .get(&lot.listing_id)
                .and_then(|m| latest_on_or_before(m, date))
            {
                Some(p) => p,
                None => continue, // No price data yet for this listing on this date.
            };

            let rate = if lot.currency_code == "EUR" {
                Decimal::ONE
            } else {
                match rates
                    .get(&lot.currency_code)
                    .and_then(|m| latest_on_or_before(m, date))
                {
                    Some(r) => r,
                    None => continue, // No FX rate available; skip rather than error.
                }
            };

            value_eur += qty * price * rate;
            cost_gross += lot.cost_contribution_eur(qty);
            cost_net += lot.cost_contribution_eur(qty) + lot.fees_eur_for_qty(qty);
        }

        // Skip dates where nothing was held or priced yet.
        if value_eur > Decimal::ZERO || cost_gross > Decimal::ZERO {
            points.push(PortfolioDataPoint {
                date,
                totals: DayTotals {
                    value_eur,
                    invested_eur: NetGross {
                        gross: cost_gross,
                        net: cost_net,
                    },
                },
            });
        }
    }
    //----------------------------

    // leftover from gemini
    // // Extract listings and currencies we care about
    // let relevant_listings: HashSet<i64> = lots.iter().map(|l| l.listing_id).collect();
    // let relevant_currencies: HashSet<String> = lots
    //     .iter()
    //     .map(|l| l.currency_code.clone())
    //     .filter(|c| c != "EUR")
    //     .collect();
    //
    // // Ensure today and start date exist in timeline
    // historical_dates.insert(start_date);
    // historical_dates.insert(today);
    //
    // let mut sorted_dates: Vec<NaiveDate> = historical_dates.into_iter().collect();
    // sorted_dates.sort();

    Ok(PortfolioHistory { points })
}
