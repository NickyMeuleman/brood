use crate::db::Db;
use crate::db::types::{InstrumentType, Replication};
use crate::isin;
use crate::{AppError, SUPPORTED_EXCHANGES};
use serde::{Deserialize, Serialize};
use specta::Type;
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
    match isin::validate(&isin) {
        Ok(_) => (),
        Err(e) => {
            dbg!(e);
        }
    };

    let row = sqlx::query!(
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
    .await?;

    Ok(row.map(|r| InstrumentDto {
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

#[derive(Debug, Deserialize, Type)]
pub struct CreateInstrumentInput {
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
pub async fn create_instrument(
    db: State<'_, Db>,
    fields: CreateInstrumentInput,
) -> Result<i64, AppError> {
    crate::isin::validate(&fields.isin)?;

    let name = fields.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::Validation("name must not be empty".into()));
    }

    let issuer = fields
        .issuer
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let domicile = fields
        .domicile
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let fsma_registered = fields.fsma_registered as i64;
    let accumulating = fields.accumulating as i64;
    let subject_to_cgt = fields.subject_to_cgt as i64;

    // fund_family_id intentionally omitted (always NULL) — family
    // resolution/creation UI is deferred.
    let result = sqlx::query!(
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
        "#,
        fields.isin,
        name,
        issuer,
        fields.instrument_type,
        fields.replication,
        fsma_registered,
        accumulating,
        domicile,
        subject_to_cgt,
    )
    .execute(&db.pool)
    .await;

    match result {
        Ok(res) => Ok(res.last_insert_rowid()),
        Err(sqlx::Error::Database(db_err))
            if db_err.message().contains("UNIQUE constraint failed") =>
        {
            Err(AppError::Validation(format!(
                "An instrument with ISIN {} already exists",
                fields.isin
            )))
        }
        Err(e) => Err(AppError::from(e)),
    }
}

#[derive(Debug, Deserialize, Type)]
pub struct CreateListingInput {
    pub instrument_id: i64,
    pub exchange_mic: String,
    pub ticker: String,
    pub currency_code: String,
    pub settlement_currency_code: Option<String>,
}

#[tauri::command]
#[specta::specta]
pub async fn create_listing(
    db: State<'_, Db>,
    fields: CreateListingInput,
) -> Result<i64, AppError> {
    if !SUPPORTED_EXCHANGES
        .iter()
        .any(|e| e.mic == fields.exchange_mic)
    {
        return Err(AppError::Validation(format!(
            "Unsupported exchange MIC '{}'",
            fields.exchange_mic
        )));
    }

    let ticker = fields.ticker.trim().to_uppercase();
    if ticker.is_empty() {
        return Err(AppError::Validation("ticker must not be empty".into()));
    }

    let currency_code = fields.currency_code.trim().to_uppercase();
    if currency_code.is_empty() {
        return Err(AppError::Validation(
            "currency_code must not be empty".into(),
        ));
    }

    let settlement_currency_code = fields
        .settlement_currency_code
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_uppercase);

    if let Some(settlement) = &settlement_currency_code {
        if *settlement == currency_code {
            return Err(AppError::Validation(
                "settlement_currency_code must differ from currency_code, or be left empty".into(),
            ));
        }
    }

    // No pre-check that instrument_id exists: it's a value the backend just
    // handed back from find_instrument_by_isin/create_instrument in this same
    // session, and the FK constraint below is the actual guard if it's ever bogus.
    let mut tx = db.pool.begin().await?;

    // currency is a bare reference table (just the code) — safe to add on
    // first use rather than requiring it to be pre-seeded.
    sqlx::query!(
        r#"INSERT INTO currency (code) VALUES (?1) ON CONFLICT (code) DO NOTHING"#,
        currency_code
    )
    .execute(&mut *tx)
    .await?;

    if let Some(settlement) = &settlement_currency_code {
        sqlx::query!(
            r#"INSERT INTO currency (code) VALUES (?1) ON CONFLICT (code) DO NOTHING"#,
            settlement
        )
        .execute(&mut *tx)
        .await?;
    }

    let result = sqlx::query!(
        r#"
        INSERT INTO listing
            (instrument_id, exchange_mic, ticker, currency_code, settlement_currency_code)
        VALUES
            (?1, ?2, ?3, ?4, ?5)
        "#,
        fields.instrument_id,
        fields.exchange_mic,
        ticker,
        currency_code,
        settlement_currency_code,
    )
    .execute(&mut *tx)
    .await;

    let listing_id = match result {
        Ok(res) => res.last_insert_rowid(),
        Err(sqlx::Error::Database(db_err))
            if db_err.message().contains("UNIQUE constraint failed") =>
        {
            return Err(AppError::Validation(format!(
                "{ticker} on {} is already registered for this instrument",
                fields.exchange_mic
            )));
        }
        Err(e) => return Err(AppError::from(e)),
    };

    tx.commit().await?;
    Ok(listing_id)
}
