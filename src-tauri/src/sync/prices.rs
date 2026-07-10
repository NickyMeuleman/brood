use crate::sync::yahoo::{Error as YahooError, fetch_prices};
use crate::{AppError, mic_timezone};
use chrono::{Days, NaiveDate, Utc};
use reqwest::Client;
use sqlx::{Pool, Sqlite, pool};
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
pub enum PriceSyncOutcome {
    Success { ticker: String, added: usize },
    Error { ticker: String, message: String },
}

pub async fn run_price_sync_tasks(
    pool: &Pool<Sqlite>,
    client: &Client,
    tasks: Vec<PriceSyncTask>,
) -> Vec<PriceSyncOutcome> {
    let mut outcomes = Vec::new();
    for (i, task) in tasks.iter().enumerate() {
        if i > 0 {
            sleep(Duration::from_millis(200)).await;
        }
        match sync_one_listing(pool, client, task).await {
            Ok(added) => outcomes.push(PriceSyncOutcome::Success {
                ticker: task.ticker.clone(),
                added,
            }),
            Err(e) => {
                eprintln!("Price sync for {} failed: {}", task.ticker, e);
                outcomes.push(PriceSyncOutcome::Error {
                    ticker: task.ticker.clone(),
                    message: e.to_string(),
                });
            }
        }
    }
    outcomes
}

pub async fn get_price_sync_tasks(pool: &Pool<Sqlite>) -> Result<Vec<PriceSyncTask>, AppError> {
    // Find all listings that have at least one open (non-closed) lot.
    // For each, determine:
    //   - The latest price date already in the DB (NULL if none).
    //   - The earliest lot acquisition date (to bound the backfill).
    let rows = sqlx::query!(
        r#"
        SELECT
            li.id               AS "listing_id!",
            li.ticker           AS "ticker!",
            li.exchange_mic     AS "exchange_mic!",
            -- 1. Get the last synced date (if any)
            (SELECT MAX(date) FROM price_history WHERE listing_id = li.id) AS "last_price_date: NaiveDate",
            -- 2. Get the earliest acquisition date across ALL lots (including closed ones)
            (
                SELECT MIN(COALESCE(DATE(t2.executed_at), DATE(ca2.effective_date)))
                FROM lot l2
                LEFT JOIN trade t2 ON t2.id = l2.source_trade_id
                LEFT JOIN corporate_action ca2 ON ca2.id = l2.source_ca_id
                WHERE l2.listing_id = li.id
            )                   AS "earliest_acquisition: NaiveDate"
        FROM listing li
        WHERE li.delisted_at IS NULL
          AND EXISTS (
              -- 3. Only sync listings that currently have at least one open lot
              SELECT 1 FROM lot l3
              LEFT JOIN lot_close lc3 ON lc3.lot_id = l3.id
              WHERE l3.listing_id = li.id AND lc3.lot_id IS NULL
          )
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
        let tz = mic_timezone(&row.exchange_mic).map_err(|e| AppError::Internal(e.to_string()))?;
        let to = Utc::now()
            .with_timezone(&tz)
            .date_naive()
            .pred_opt()
            .expect("valid date");

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

pub async fn get_full_price_sync_tasks(
    pool: &Pool<Sqlite>,
) -> Result<Vec<PriceSyncTask>, AppError> {
    // 1. Get every active listing that has at least one lot
    let rows = sqlx::query!(
        r#"
        SELECT 
            li.id as "id!",
            li.ticker as "ticker!",
            li.exchange_mic as "mic!",
            (
                SELECT MIN(COALESCE(DATE(t.executed_at), DATE(ca.effective_date)))
                FROM lot l
                LEFT JOIN trade t ON t.id = l.source_trade_id
                LEFT JOIN corporate_action ca ON ca.id = l.source_ca_id
                WHERE l.listing_id = li.id
            )                   AS "earliest: NaiveDate"
        FROM listing li
        WHERE EXISTS (SELECT 1 FROM lot l WHERE l.listing_id = li.id)
          AND li.delisted_at IS NULL
        "#
    )
    .fetch_all(pool)
    .await?;

    let tasks = rows
        .into_iter()
        .map(|row| {
            let tz = mic_timezone(&row.mic).map_err(|e| AppError::Internal(e.to_string()))?;
            let to = Utc::now()
                .with_timezone(&tz)
                .date_naive()
                .pred_opt()
                .expect("valid date");
            let from = row.earliest.unwrap_or(to);
            Ok(PriceSyncTask {
                listing_id: row.id,
                ticker: row.ticker,
                mic: row.mic,
                from,
                to,
            })
        })
        .collect::<Result<Vec<PriceSyncTask>, AppError>>()?;

    Ok(tasks)
}

/// Sync prices for all listings that have open lots.
/// Skips listings that are already up to date (last stored date = yesterday).
/// Fails gracefully per-listing
pub async fn sync_all_prices(
    pool: &Pool<Sqlite>,
    client: &Client,
) -> Result<Vec<PriceSyncOutcome>, AppError> {
    let tasks = get_price_sync_tasks(pool).await?;
    let outcomes = run_price_sync_tasks(pool, client, tasks).await;
    Ok(outcomes)
}

pub async fn sync_one_listing(
    pool: &Pool<Sqlite>,
    client: &Client,
    task: &PriceSyncTask,
) -> Result<usize, AppError> {
    let bars = fetch_prices(client, &task.ticker, &task.mic, task.from, task.to)
        .await
        .map_err(|e| match e {
            YahooError::UnknownMic(msg) => AppError::Internal(msg),
            other => AppError::ExternalService(other.to_string()),
        })?;

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
            ON CONFLICT (listing_id, date, source) DO UPDATE SET
              open   = excluded.open,
              high   = excluded.high,
              low    = excluded.low,
              close  = excluded.close,
              volume = excluded.volume
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

pub async fn force_sync_one_listing(
    pool: &Pool<Sqlite>,
    client: &Client,
    listing_id: i64,
    mic: String,
    ticker: String,
) -> Result<PriceSyncOutcome, AppError> {
    let tz = mic_timezone(&mic).map_err(|e| AppError::Internal(e.to_string()))?;
    let to = Utc::now()
        .with_timezone(&tz)
        .date_naive()
        .pred_opt()
        .expect("valid date");

    // get earliest data, fallback to yesterday if not found
    let from = sqlx::query_scalar!(
        r#"
        SELECT MIN(COALESCE(DATE(t.executed_at), DATE(ca.effective_date))) as "d: NaiveDate"
        FROM lot l
        LEFT JOIN trade t ON t.id = l.source_trade_id
        LEFT JOIN corporate_action ca ON ca.id = l.source_ca_id
        WHERE l.listing_id = ?
        "#,
        listing_id
    )
    .fetch_one(pool)
    .await?
    .unwrap_or(to);

    let task = PriceSyncTask {
        listing_id,
        ticker: ticker.clone(),
        mic,
        from,
        to,
    };

    let added = sync_one_listing(pool, client, &task).await?;
    Ok(PriceSyncOutcome::Success { ticker, added })
}

pub async fn force_sync_all_prices(
    pool: &Pool<Sqlite>,
    client: &Client,
) -> Result<Vec<PriceSyncOutcome>, AppError> {
    let tasks = get_full_price_sync_tasks(pool).await?;
    let outcomes = run_price_sync_tasks(pool, client, tasks).await;
    Ok(outcomes)
}
