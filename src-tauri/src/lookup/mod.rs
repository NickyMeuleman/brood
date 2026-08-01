use crate::{
    AppError,
    db::types::InstrumentType,
    lookup::{gleif::search_gleif, openfigi::ListingCandidate},
};
use reqwest::Client;

pub mod gleif;
pub mod openfigi;
pub mod yahoo;

pub fn guess_instrument_type(candidate: &ListingCandidate) -> Option<InstrumentType> {
    // Catch ETFs first (Bloomberg puts them in the "Equity" sector)
    if candidate.security_type.as_deref() == Some("ETP") {
        return Some(InstrumentType::Etf);
    }

    match candidate.market_sector.as_deref() {
        Some("Equity" | "Pfd") => Some(InstrumentType::Stock),
        Some("Govt" | "Corp" | "Mtge" | "Muni") => Some(InstrumentType::Bond),
        _ => None,
    }
}

pub fn guess_accumulating(name: &str) -> Option<bool> {
    let name = name.to_uppercase();
    let words: Vec<_> = name.split(|c: char| !c.is_alphanumeric()).collect();

    let is_acc = words
        .iter()
        .any(|&w| matches!(w, "ACC" | "AC" | "ACCUMULATING"));
    let is_dist = words
        .iter()
        .any(|&w| matches!(w, "DIS" | "DIST" | "DISTRIBUTING"));

    match (is_acc, is_dist) {
        (true, false) => Some(true),
        (false, true) => Some(false),
        _ => None,
    }
}

/// For ETFs the first 2 letters of the isin are the domicile,
/// this does not hold for stocks
pub async fn guess_domicile(
    isin: &str,
    instrument_type: &Option<InstrumentType>,
    client: &Client,
) -> Result<Option<String>, AppError> {
    if matches!(
        instrument_type,
        Some(InstrumentType::Etf) | Some(InstrumentType::Fund)
    ) {
        let prefix = isin
            .get(0..2)
            .ok_or(AppError::Validation("ISIN format invalid".into()))?;
        return Ok(prefix
            .chars()
            .all(|c| c.is_ascii_uppercase())
            .then(|| prefix.to_string()));
    }
    let gleif_info = search_gleif(client, isin)
        .await
        .map_err(|e| AppError::ExternalService(format!("GLEIF Domicile lookup failed: {e}")))?;

    Ok(Some(gleif_info.domicile))
}

pub async fn guess_issuer(isin: &str, client: &Client) -> Result<Option<String>, AppError> {
    let gleif_info = search_gleif(client, isin)
        .await
        .map_err(|e| AppError::ExternalService(format!("GLEIF Domicile lookup failed: {e}")))?;

    Ok(Some(gleif_info.legal_name))
}
