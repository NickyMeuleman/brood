use crate::db::Db;
use crate::sync::fx::{sync_all_fx, FXSyncOutcome};
use crate::sync::prices::{sync_all_prices, sync_one_listing, PriceSyncOutcome, PriceSyncTask};
use crate::{mic_timezone, AppError};
use chrono::{NaiveDate, Utc};
use tauri::State;

#[tauri::command]
#[specta::specta]
pub async fn sync_prices(db: State<'_, Db>) -> Result<Vec<PriceSyncOutcome>, AppError> {
    let client = reqwest::Client::new();
    let outcomes = sync_all_prices(&db.pool, &client).await?;
    dbg!("price outcomes: ", &outcomes);
    Ok(outcomes)
}

#[tauri::command]
#[specta::specta]
pub async fn sync_fx(db: State<'_, Db>) -> Result<Vec<FXSyncOutcome>, AppError> {
    let client = reqwest::Client::new();
    let outcomes = sync_all_fx(&db.pool, &client).await?;
    dbg!("Fx outcomes: ", &outcomes);
    Ok(outcomes)
}

#[tauri::command]
#[specta::specta]
pub async fn force_update_one_listing_prices(
    db: State<'_, Db>,
    listing_id: i64,
    mic: String,
    ticker: String,
) -> Result<usize, AppError> {
    let client = reqwest::Client::new();

    let tz = mic_timezone(&mic).map_err(|_| AppError::Internal)?;
    let to = Utc::now()
        .with_timezone(&tz)
        .date_naive()
        .pred_opt()
        .unwrap();

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
    .fetch_one(&db.pool)
    .await?
    .unwrap_or(to);

    let task = PriceSyncTask {
        listing_id,
        ticker,
        mic,
        from,
        to,
    };

    sync_one_listing(&db.pool, &client, &task).await
}

#[tauri::command]
#[specta::specta]
pub async fn force_update_all_prices(db: State<'_, Db>) -> Result<Vec<PriceSyncOutcome>, AppError> {
    let client = reqwest::Client::new();

    // 1. Get every active listing that has at least one lot
    let rows = sqlx::query!(
        r#"
        SELECT 
            li.id as "id!", li.ticker as "ticker!", li.exchange_mic as "mic!",
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
    .fetch_all(&db.pool)
    .await?;

    let mut outcomes = Vec::new();
    for (i, row) in rows.into_iter().enumerate() {
        // Respect Yahoo's API limits
        if i > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }

        let tz = mic_timezone(&row.mic).map_err(|_| AppError::Internal)?;
        let to = Utc::now()
            .with_timezone(&tz)
            .date_naive()
            .pred_opt()
            .unwrap();
        let from = row.earliest.unwrap_or(to);

        let task = PriceSyncTask {
            listing_id: row.id,
            ticker: row.ticker.clone(),
            mic: row.mic,
            from,
            to,
        };

        match sync_one_listing(&db.pool, &client, &task).await {
            Ok(added) => outcomes.push(PriceSyncOutcome::Success {
                ticker: row.ticker,
                added,
            }),
            Err(e) => outcomes.push(PriceSyncOutcome::Error {
                ticker: row.ticker,
                message: e.to_string(),
            }),
        }
    }

    Ok(outcomes)
}
