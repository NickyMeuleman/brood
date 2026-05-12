use chrono::NaiveDate;
use reqwest::Client;
use rust_decimal::Decimal;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Rate {
    pub date: NaiveDate,
    pub base: String,
    pub quote: String,
    pub rate: Decimal,
}

pub async fn fetch_rates(
    client: &Client,
    currency: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<Rate>, reqwest::Error> {
    let url = format!(
        "https://api.frankfurter.dev/v2/rates?base={}&quotes=EUR&from={}&to={}&providers=ECB",
        currency,
        from.format("%Y-%m-%d"),
        to.format("%Y-%m-%d")
    );
    let response = client.get(&url).send().await?.json().await?;

    // nog dingen doen!
    Ok(response)
}
