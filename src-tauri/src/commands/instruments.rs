use crate::db::Db;
use crate::db::types::{InstrumentType, Replication};
use crate::isin;
use crate::{AppError, SUPPORTED_EXCHANGES};
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqliteConnection;
use tauri::State;

#[derive(Debug, Serialize, Type)]
pub struct InstrumentDto {
    pub id: i64,
    pub isin: String,
    pub name: String,
    pub issuer: Option<String>,
    pub instrument_type: InstrumentType,
    pub replication: Option<Replication>,
    pub fsma_registered: bool,
    pub accumulating: bool,
    pub domicile: Option<String>,
    pub subject_to_cgt: bool,
}

#[tauri::command]
#[specta::specta]
pub async fn find_instrument_by_isin(
    db: State<'_, Db>,
    isin: String,
) -> Result<Option<InstrumentDto>, AppError> {
    isin::validate(&isin)?;

    Ok(sqlx::query!(
        r#"
        SELECT
            id                AS "id!",
            isin              AS "isin!",
            name              AS "name!",
            issuer,
            instrument_type   AS "instrument_type!: InstrumentType",
            replication       AS "replication: Replication",
            fsma_registered   AS "fsma_registered!",
            accumulating      AS "accumulating!",
            domicile,
            subject_to_cgt    AS "subject_to_cgt!"
        FROM instrument
        WHERE isin = ?1
        "#,
        isin
    )
    .fetch_optional(&db.pool)
    .await?
    .map(|r| InstrumentDto {
        id: r.id,
        isin: r.isin,
        name: r.name,
        issuer: r.issuer,
        instrument_type: r.instrument_type,
        replication: r.replication,
        fsma_registered: r.fsma_registered != 0,
        accumulating: r.accumulating != 0,
        domicile: r.domicile,
        subject_to_cgt: r.subject_to_cgt != 0,
    }))
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct InstrumentDetails {
    pub isin: String,
    pub name: String,
    pub issuer: String,
    pub instrument_type: InstrumentType,
    pub replication: Option<Replication>,
    pub fsma_registered: bool,
    pub accumulating: bool,
    pub domicile: String,
    pub subject_to_cgt: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ListingDetails {
    pub mic: String,
    pub ticker: String,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "kind")]
pub enum AddListingInput {
    NewInstrument {
        instrument: InstrumentDetails,
        listing: ListingDetails,
    },
    ExistingInstrument {
        instrument_id: i64,
        listing: ListingDetails,
    },
    UpdateInstrument {
        instrument_id: i64,
        instrument: InstrumentDetails,
        listing: ListingDetails,
    },
}

#[tauri::command]
#[specta::specta]
pub async fn add_listing_form(db: State<'_, Db>, input: AddListingInput) -> Result<i64, AppError> {
    let mut tx = db.pool.begin().await?;

    let (instrument_id, listing) = match input {
        AddListingInput::NewInstrument {
            instrument,
            listing,
        } => {
            let id = add_instrument(&mut tx, instrument).await?;
            (id, listing)
        }
        AddListingInput::ExistingInstrument {
            instrument_id,
            listing,
        } => (instrument_id, listing),
        AddListingInput::UpdateInstrument {
            instrument_id,
            instrument,
            listing,
        } => {
            crate::isin::validate(&instrument.isin)?;
            (instrument_id, listing)
        }
    };

    let listing_id = add_listing(&mut tx, instrument_id, listing).await?;
    tx.commit().await?;

    Ok(listing_id)
}

async fn add_instrument(
    conn: &mut SqliteConnection,
    instrument: InstrumentDetails,
) -> Result<i64, AppError> {
    crate::isin::validate(&instrument.isin)?;

    let name = instrument.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::Validation("name must not be empty".into()));
    }
    let issuer = instrument.issuer.trim().to_string();
    if issuer.is_empty() {
        return Err(AppError::Validation("issuer must not be empty".into()));
    }
    let domicile = instrument.domicile.trim().to_string();
    if domicile.is_empty() {
        return Err(AppError::Validation("domicile must not be empty".into()));
    }

    sqlx::query_scalar!(
        r#"
        INSERT INTO instrument
            (isin,
            name,
            issuer,
            instrument_type,
            replication,
            fsma_registered,
            accumulating,
            domicile,
            subject_to_cgt)
        VALUES
            (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        RETURNING id
        "#,
        instrument.isin,
        name,
        issuer,
        instrument.instrument_type,
        instrument.replication,
        instrument.fsma_registered,
        instrument.accumulating,
        domicile,
        instrument.subject_to_cgt,
    )
    .fetch_one(&mut *conn)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(db_err) if db_err.is_unique_violation() => AppError::Validation(
            format!("An instrument with ISIN {} already exists", instrument.isin),
        ),
        e => AppError::from(e),
    })
}

async fn add_listing(
    conn: &mut SqliteConnection,
    instrument_id: i64,
    listing: ListingDetails,
) -> Result<i64, AppError> {
    if !SUPPORTED_EXCHANGES.iter().any(|e| e.mic == listing.mic) {
        return Err(AppError::Validation(format!(
            "Unsupported exchange MIC '{}'",
            listing.mic
        )));
    }

    let ticker = listing.ticker.trim().to_uppercase();
    if ticker.is_empty() {
        return Err(AppError::Validation("ticker must not be empty".into()));
    }

    let currency_code = listing.currency.trim().to_uppercase();
    if currency_code.is_empty() {
        return Err(AppError::Validation("currency must not be empty".into()));
    }

    sqlx::query!(
        r#"
        INSERT INTO currency (code) VALUES (?1) ON CONFLICT (code) DO NOTHING
        "#,
        currency_code
    )
    .execute(&mut *conn)
    .await?;

    sqlx::query_scalar!(
        r#"
        INSERT INTO listing (
            instrument_id,
            exchange_mic,
            ticker,
            currency_code)
        VALUES (?1, ?2, ?3, ?4)
        RETURNING id as "id!"
        "#,
        instrument_id,
        listing.mic,
        ticker,
        currency_code
    )
    .fetch_one(&mut *conn)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
            AppError::Validation(format!(
                "A listing for ticker {} on exchange {} already exists",
                ticker, listing.mic
            ))
        }
        e => AppError::from(e),
    })
}
