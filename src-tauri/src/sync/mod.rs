pub mod ecb;
pub mod fx;
pub mod prices;
pub mod yahoo;

use crate::AppError;
use crate::sync::{fx::force_sync_one_currency, prices::force_sync_one_listing};
use reqwest::Client;
use sqlx::{Pool, Sqlite};

pub async fn backfill(
    pool: &Pool<Sqlite>,
    client: &Client,
    listing_id: i64,
) -> Result<(), AppError> {
    let listing = sqlx::query!(
        r#"
        SELECT
            ticker,
            exchange_mic,
            currency_code
        FROM listing
        WHERE id = ?1
        "#,
        listing_id
    )
    .fetch_one(pool)
    .await?;

    force_sync_one_listing(
        pool,
        client,
        listing_id,
        listing.exchange_mic,
        listing.ticker,
    )
    .await?;

    if listing.currency_code != "EUR" {
        force_sync_one_currency(pool, client, listing.currency_code).await?;
    }

    Ok(())
}
