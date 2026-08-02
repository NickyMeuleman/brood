use crate::db::Db;
use crate::db::types::{InstrumentType, Replication};
use crate::lookup::openfigi::{ListingCandidate, search_isin_listings};
use crate::lookup::yahoo::get_listing_meta;
use crate::lookup::{guess_accumulating, guess_domicile, guess_instrument_type, guess_issuer};
use crate::{
    AppError, HttpClient, SUPPORTED_EXCHANGES, isin, sanitize_currency, sanitize_domicile,
    sanitize_mic, sanitize_string, sanitize_ticker,
};
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
    pub issuer: Option<String>,
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
            update_instrument(&mut tx, instrument_id, instrument).await?;
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
    let isin = instrument.isin.trim().to_uppercase();
    crate::isin::validate(&isin)?;

    let name = sanitize_string(&instrument.name, "name")?;
    let issuer = instrument
        .issuer
        .map(|issuer| sanitize_string(&issuer, "issuer"))
        .transpose()?;
    let domicile = sanitize_domicile(&instrument.domicile)?;

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
        isin,
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

async fn update_instrument(
    conn: &mut SqliteConnection,
    instrument_id: i64,
    instrument: InstrumentDetails,
) -> Result<(), AppError> {
    let isin = instrument.isin.trim().to_uppercase();
    isin::validate(&isin)?;
    let name = sanitize_string(&instrument.name, "name")?;
    let issuer = instrument
        .issuer
        .map(|issuer| sanitize_string(&issuer, "issuer"))
        .transpose()?;
    let domicile = sanitize_domicile(&instrument.domicile)?;

    let rows = sqlx::query!(
        r#"
        UPDATE instrument
        SET 
          isin = ?1,
          name = ?2,
          issuer = ?3,
          instrument_type = ?4, 
          replication = ?5,
          fsma_registered = ?6,
          accumulating = ?7, 
          domicile = ?8,
          subject_to_cgt = ?9
        WHERE id = ?10
        "#,
        isin,
        name,
        issuer,
        instrument.instrument_type,
        instrument.replication,
        instrument.fsma_registered,
        instrument.accumulating,
        domicile,
        instrument.subject_to_cgt,
        instrument_id
    )
    .execute(&mut *conn)
    .await?
    .rows_affected();

    if rows == 0 {
        return Err(AppError::Validation(
            "Instrument not found for update".into(),
        ));
    }
    Ok(())
}

async fn add_listing(
    conn: &mut SqliteConnection,
    instrument_id: i64,
    listing: ListingDetails,
) -> Result<i64, AppError> {
    let mic = sanitize_mic(&listing.mic)?;
    let ticker = sanitize_ticker(&listing.ticker)?;
    let currency_code = sanitize_currency(&listing.currency)?;

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
        mic,
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

#[derive(Debug, Serialize, Type)]
pub struct ListingSearchResult {
    pub candidates: Vec<ListingCandidate>,
    pub instrument_type_hint: Option<InstrumentType>,
    pub accumulating_hint: Option<bool>,
    pub domicile_hint: Option<String>,
    pub issuer_hint: Option<String>,
}

#[tauri::command]
#[specta::specta]
pub async fn find_listings_by_isin(
    http: State<'_, HttpClient>,
    isin: String,
) -> Result<ListingSearchResult, AppError> {
    let mics: Vec<&str> = SUPPORTED_EXCHANGES.iter().map(|e| e.mic).collect();

    let candidates = search_isin_listings(&http.client, &isin, &mics)
        .await
        .map_err(|e| AppError::ExternalService(e.to_string()))?;

    let first = candidates.first();
    let instrument_type_hint = first.and_then(guess_instrument_type);
    let accumulating_hint = first.and_then(|item| guess_accumulating(&item.name));
    // PERF: two requests for the same thing, bleh
    let domicile_hint = match guess_domicile(&isin, &instrument_type_hint, &http.client).await {
        Ok(hint) => hint,
        Err(e) => {
            eprintln!("Domicile hint lookup failed for {isin} (non-fatal): {e}");
            None
        }
    };
    let issuer_hint = match guess_issuer(&isin, &http.client).await {
        Ok(hint) => hint,
        Err(e) => {
            eprintln!("Issuer hint lookup failed for {isin} (non-fatal): {e}");
            None
        }
    };

    Ok(ListingSearchResult {
        instrument_type_hint,
        accumulating_hint,
        domicile_hint,
        issuer_hint,
        candidates,
    })
}

#[derive(Debug, Serialize, Type)]
pub struct MetaHints {
    pub currency: String,
    pub name: Option<String>,
    /// Re-checked against Yahoo's fuller name.
    /// (Already done with FIGI name, but this name is better)
    pub accumulating_hint: Option<bool>,
}

#[tauri::command]
#[specta::specta]
pub async fn listing_meta(
    http: State<'_, HttpClient>,
    ticker: String,
    mic: String,
) -> Result<MetaHints, AppError> {
    let meta = get_listing_meta(&http.client, &ticker, &mic)
        .await
        .map_err(|e| AppError::ExternalService(e.to_string()))?;
    let accumulating_hint = meta
        .long_name
        .as_ref()
        .and_then(|name| guess_accumulating(name));
    let name = meta.long_name;
    let currency = meta.currency.ok_or_else(|| {
        AppError::ExternalService(format!("No currency was found for {ticker} on {mic}"))
    })?;

    Ok(MetaHints {
        currency,
        name,
        accumulating_hint,
    })
}
