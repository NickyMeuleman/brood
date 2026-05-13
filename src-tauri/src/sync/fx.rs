use crate::{sync::ecb::fetch_rates, AppError};
use chrono::{Days, NaiveDate, Utc};
use reqwest::Client;
use sqlx::{Pool, Sqlite};
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug)]
pub struct FxSyncTask {
    pub currency: String,
    /// Both dates are inclusive.
    /// For a new currency this is derived from the earliest lot acquisition date
    /// of any listing denominated in that currency.
    /// For an existing currency this is MAX(fx_rate.date) + 1 day.
    pub from: NaiveDate,
    pub to: NaiveDate,
}

#[derive(Debug, serde::Serialize, specta::Type)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum FXSyncOutcome {
    Success { currency: String, added: usize },
    Error { currency: String, message: String },
}

pub async fn sync_all_fx(
    pool: &Pool<Sqlite>,
    client: &Client,
) -> Result<Vec<FXSyncOutcome>, AppError> {
    let tasks = get_fx_sync_tasks(pool).await?;

    let mut outcomes = Vec::new();
    for (i, task) in tasks.iter().enumerate() {
        // be nice to the Frankfurter API to not get rate limited
        if i > 0 {
            sleep(Duration::from_millis(200)).await;
        }

        match sync_one_currency(pool, client, task).await {
            Ok(added) => outcomes.push(FXSyncOutcome::Success {
                currency: task.currency.clone(),
                added,
            }),
            Err(e) => {
                eprintln!("FX sync for {} failed: {}", task.currency, e);
                outcomes.push(FXSyncOutcome::Error {
                    currency: task.currency.clone(),
                    message: e.to_string(),
                });
            }
        }
    }

    Ok(outcomes)
}

pub async fn get_fx_sync_tasks(pool: &Pool<Sqlite>) -> Result<Vec<FxSyncTask>, AppError> {
    // Find all non-EUR currencies used by active listings that have open lots.
    // For each, determine:
    //   - The latest rate date already in the DB for that currency (NULL if none).
    //   - The earliest lot acquisition date for any listing in that currency,
    //     to bound the initial backfill.
    //
    // aggregate across all listings sharing the same currency code.
    let rows = sqlx::query!(
        r#"
        SELECT
            li.currency_code                                                    AS "currency!",
            -- 1. Last stored ECB rate for this currency
            (
                SELECT MAX(date) FROM fx_rate
                WHERE currency = li.currency_code AND source = 'ECB'
            )                                                                   AS "last_rate_date: NaiveDate",
            -- 2. Earliest acquisition across all lots for any listing in this currency
            (
                SELECT MIN(COALESCE(DATE(t2.executed_at), DATE(ca2.effective_date)))
                FROM lot l2
                LEFT JOIN listing li2 ON li2.id = l2.listing_id
                LEFT JOIN trade t2 ON t2.id = l2.source_trade_id
                LEFT JOIN corporate_action ca2 ON ca2.id = l2.source_ca_id
                WHERE li2.currency_code = li.currency_code
            )                                                                   AS "earliest_acquisition: NaiveDate"
        FROM listing li
        WHERE li.currency_code != 'EUR'
          AND li.delisted_at IS NULL
          AND EXISTS (
              SELECT 1 FROM lot l3
              LEFT JOIN lot_close lc3 ON lc3.lot_id = l3.id
              WHERE l3.listing_id = li.id AND lc3.lot_id IS NULL
          )
        GROUP BY li.currency_code
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::from)?;

    // The ECB publishes rates once per business day with no timezone component.
    let to = Utc::now().date_naive().pred_opt().expect("valid date");

    let mut tasks = Vec::new();
    for row in rows {
        let from = match (row.last_rate_date, row.earliest_acquisition) {
            (Some(last), _) => last.checked_add_days(Days::new(1)).expect("valid date"),
            (_, Some(earliest)) => earliest,
            (None, None) => continue,
        };

        if from <= to {
            tasks.push(FxSyncTask {
                currency: row.currency,
                from,
                to,
            });
        }
    }

    Ok(tasks)
}

pub async fn sync_one_currency(
    pool: &Pool<Sqlite>,
    client: &Client,
    task: &FxSyncTask,
) -> Result<usize, AppError> {
    let rates = fetch_rates(client, &task.currency, task.from, task.to)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    if rates.is_empty() {
        // the gap is entirely ECB non-business days.
        return Ok(0);
    }

    let mut tx = pool.begin().await.map_err(AppError::from)?;
    let mut count = 0;

    for rate in rates {
        let currency = &rate.currency;
        let date = rate.date;
        let rate_to_eur = rate.rate_to_eur.to_string();
        let source = rate.source;

        let affected = sqlx::query!(
            r#"
            INSERT INTO fx_rate (date, currency, rate_to_eur, source)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT (date, currency, source) DO UPDATE SET
                rate_to_eur = excluded.rate_to_eur
            "#,
            date,
            currency,
            rate_to_eur,
            source
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
