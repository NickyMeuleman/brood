use crate::commands::lot_data::load_lot_records;
use crate::commands::{
    get_prices, get_rates, latest_on_or_before, period_start, DayTotals, NetGross, Period,
};
use crate::{db::Db, AppError};
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::Serialize;
use specta::Type;
use std::collections::{BTreeSet, HashSet};
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
    let prices = get_prices(&db.pool, today).await?;

    // Load the full FX rate history up to today for the same reason.
    let rates = get_rates(&db.pool).await?;

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

    Ok(PortfolioHistory { points })
}
