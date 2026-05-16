use std::time::Duration;

use crate::db::Db;
use crate::sync::fx::{sync_all_fx, FXSyncOutcome};
use crate::sync::prices::{sync_all_prices, sync_one_listing, PriceSyncOutcome, PriceSyncTask};
use crate::{mic_timezone, AppError, HttpClient};
use chrono::{NaiveDate, Utc};
use tauri::State;
use tokio;

#[derive(Debug, serde::Serialize, specta::Type)]
#[serde(tag = "status", rename_all = "camelCase")]
pub struct SyncOutcomes {
    fx: Vec<FXSyncOutcome>,
    prices: Vec<PriceSyncOutcome>,
}

#[tauri::command]
#[specta::specta]
pub async fn sync(
    db: State<'_, Db>,
    http: State<'_, HttpClient>,
) -> Result<SyncOutcomes, AppError> {
    let sync_task = async {
        let (fx_outcomes, prices_outcomes) = tokio::try_join!(
            sync_all_fx(&db.pool, &http.client),
            sync_all_prices(&db.pool, &http.client)
        )?;

        Ok(SyncOutcomes {
            fx: fx_outcomes,
            prices: prices_outcomes,
        })
    };

    match tokio::time::timeout(Duration::from_secs(15), sync_task).await {
        Ok(result) => result,
        Err(_) => Err(AppError::Timeout("Sync timed out after 15s".into())),
    }
}

#[tauri::command]
#[specta::specta]
pub async fn sync_prices(
    db: State<'_, Db>,
    http: State<'_, HttpClient>,
) -> Result<Vec<PriceSyncOutcome>, AppError> {
    let outcomes = sync_all_prices(&db.pool, &http.client).await?;
    Ok(outcomes)
}

#[tauri::command]
#[specta::specta]
pub async fn sync_fx(
    db: State<'_, Db>,
    http: State<'_, HttpClient>,
) -> Result<Vec<FXSyncOutcome>, AppError> {
    let outcomes = sync_all_fx(&db.pool, &http.client).await?;
    Ok(outcomes)
}

#[tauri::command]
#[specta::specta]
pub async fn force_update_one_listing_prices(
    db: State<'_, Db>,
    http: State<'_, HttpClient>,
    listing_id: i64,
    mic: String,
    ticker: String,
) -> Result<usize, AppError> {
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

    sync_one_listing(&db.pool, &http.client, &task).await
}

#[tauri::command]
#[specta::specta]
pub async fn force_update_all_prices(
    db: State<'_, Db>,
    http: State<'_, HttpClient>,
) -> Result<Vec<PriceSyncOutcome>, AppError> {
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

        match sync_one_listing(&db.pool, &http.client, &task).await {
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
