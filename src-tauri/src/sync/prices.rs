use crate::sync::yahoo::fetch_prices;
use crate::{mic_timezone, AppError};
use chrono::{Days, NaiveDate, Utc};
use reqwest::Client;
use sqlx::{Pool, Sqlite};
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug)]
pub struct PriceSyncTask {
    pub listing_id: i64,
    pub ticker: String,
    pub mic: String,
    /// Both dates are inclusive and local to the exchange
    /// The earliest date we need prices from.
    /// For a brand-new listing this is derived from the first lot acquisition date.
    /// For an existing listing this is MAX(price_history.date) + 1 day.
    pub from: NaiveDate,
    pub to: NaiveDate,
}

#[derive(Debug, serde::Serialize, specta::Type)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum SyncOutcome {
    Success { ticker: String, added: usize },
    Error { ticker: String, message: String },
}

/// Sync prices for all listings that have open lots.
/// Skips listings that are already up to date (last stored date = yesterday).
/// Fails gracefully per-listing
pub async fn sync_all_prices(
    pool: &Pool<Sqlite>,
    client: &Client,
) -> Result<Vec<SyncOutcome>, AppError> {
    let price_sync_tasks = get_price_sync_tasks(pool).await?;

    let mut outcomes = Vec::new();
    for (i, task) in price_sync_tasks.iter().enumerate() {
        // be nice to the Yahoo API to not get rate limited
        if i > 0 {
            sleep(Duration::from_millis(200)).await;
        }

        match sync_one_listing(pool, client, task).await {
            Ok(added) => outcomes.push(SyncOutcome::Success {
                ticker: task.ticker.clone(),
                added,
            }),
            Err(e) => {
                eprintln!("Price sync for {} failed: {}", task.ticker, e);
                outcomes.push(SyncOutcome::Error {
                    ticker: task.ticker.clone(),
                    message: e.to_string(),
                });
            }
        }
    }

    Ok(outcomes)
}

pub async fn get_price_sync_tasks(pool: &Pool<Sqlite>) -> Result<Vec<PriceSyncTask>, AppError> {
    // Find all listings that have at least one open (non-closed) lot.
    // For each, determine:
    //   - The latest price date already in the DB (NULL if none).
    //   - The earliest lot acquisition date (to bound the backfill).
    let rows = sqlx::query!(
        r#"
        SELECT
            li.id              AS "listing_id!",
            li.ticker          AS "ticker!",
            li.exchange_mic    AS "exchange_mic!",
            MAX(ph.date)       AS "last_price_date: NaiveDate",
            -- Earliest acquisition: trade-originated lots use trade.executed_at,
            -- CA-originated lots fall back to the corporate_action.effective_date.
            MIN(
                COALESCE(
                    DATE(t.executed_at),
                    DATE(ca.effective_date)
                )
            )                  AS "earliest_acquisition: NaiveDate"
        FROM listing li
        JOIN lot l ON l.listing_id = li.id
        LEFT JOIN lot_close lc ON lc.lot_id = l.id
        LEFT JOIN trade t ON t.id = l.source_trade_id
        LEFT JOIN corporate_action ca ON ca.id = l.source_ca_id
        LEFT JOIN price_history ph ON ph.listing_id = li.id
        WHERE lc.lot_id IS NULL          -- open lots only
          AND li.delisted_at IS NULL     -- active listings only
        GROUP BY li.id, li.ticker, li.exchange_mic
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::from)?;

    let mut tasks = Vec::new();
    for row in rows {
        let from = match (row.last_price_date, row.earliest_acquisition) {
            (Some(last), _) => last.checked_add_days(Days::new(1)).expect("Valid day"),
            (_, Some(earliest)) => earliest,
            (None, None) => continue,
        };
        let tz = mic_timezone(&row.exchange_mic).map_err(|_| AppError::Internal)?;
        let to = Utc::now()
            .with_timezone(&tz)
            .date_naive()
            .pred_opt()
            .expect("Valid starting date");

        if from <= to {
            tasks.push(PriceSyncTask {
                listing_id: row.listing_id,
                ticker: row.ticker,
                mic: row.exchange_mic,
                to,
                from,
            });
        }
    }

    Ok(tasks)
}

pub async fn sync_one_listing(
    pool: &Pool<Sqlite>,
    client: &Client,
    task: &PriceSyncTask,
) -> Result<usize, AppError> {
    let bars = fetch_prices(client, &task.ticker, &task.mic, task.from, task.to)
        .await
        .map_err(|_| AppError::Internal)?;

    if bars.is_empty() {
        return Ok(0);
    }

    let mut tx = pool.begin().await.map_err(AppError::from)?;
    let mut count = 0;

    for bar in bars {
        let id = task.listing_id;
        let date = bar.date;
        let open = bar.open.to_string();
        let high = bar.high.to_string();
        let low = bar.low.to_string();
        let close = bar.close.to_string();
        let volume = bar.volume as i64;
        let affected = sqlx::query!(
            r#"
            INSERT INTO price_history (listing_id, date, open, high, low, close, volume, source)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'YAHOO')
            ON CONFLICT (listing_id, date, source) DO NOTHING
            "#,
            id,
            date,
            open,
            high,
            low,
            close,
            volume
        )
        .execute(&mut *tx)
        .await
        .map_err(AppError::from)?
        .rows_affected();

        count += affected as usize;
    }

    tx.commit().await.map_err(AppError::from)?;
    Ok(count)
}

// async fn insert_price_bars_bulk(
//     pool: &Pool<Sqlite>,
//     listing_id: i64,
//     bars: &[PriceBar],
// ) -> Result<usize, AppError> {
//     // sqlx::QueryBuilder creates a single parameterized batch insert (much faster than looping)
//     let mut query_builder = QueryBuilder::new(
//         "INSERT INTO price_history (listing_id, date, source, close, open, high, low, volume) "
//     );
//
//     query_builder.push_values(bars, |mut b, bar| {
//         b.push_bind(listing_id)
//          .push_bind(bar.date)
//          .push_bind("EXCHANGE") // Must match schema CHECK constraint!
//          .push_bind(bar.close.to_string())
//          .push_bind(bar.open.to_string())
//          .push_bind(bar.high.to_string())
//          .push_bind(bar.low.to_string())
//          .push_bind(bar.volume as i64);
//     });
//
//     // ON CONFLICT DO UPDATE ensures we overwrite existing data if Yahoo retroactively adjusted prices
//     query_builder.push(
//         r#"
//         ON CONFLICT(listing_id, date, source)
//         DO UPDATE SET
//             close = excluded.close,
//             open = excluded.open,
//             high = excluded.high,
//             low = excluded.low,
//             volume = excluded.volume
//         "#
//     );
//
//     let rows_affected = query_builder
//         .build()
//         .execute(pool)
//         .await
//         .map_err(AppError::from)?
//         .rows_affected();
//
//     // Note: Due to SQLite mechanics, DO UPDATE might report 2 rows affected for a single update.
//     // If you need exact counts of bars processed, just return `bars.len()`.
//     Ok(bars.len())
// }
