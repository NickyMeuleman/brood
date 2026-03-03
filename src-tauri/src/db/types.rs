// Internal database types. No main types here cross the Tauri
// command boundary. Response structs with serde/specta derives
// are defined alongside the commands that use them.
//
// Decimal values are stored as TEXT in SQLite. sqlx cannot decode
// TEXT into rust_decimal::Decimal directly, so all decimal fields
// are String here. Parse to Decimal when arithmetic is needed.
use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;

// ============================================================
// Enums
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, sqlx::Type, Serialize, specta::Type)]
#[sqlx(type_name = "TEXT", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InstrumentType {
    Etf,
    Fund,
    Stock,
    Bond,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::Type, Serialize, specta::Type)]
#[sqlx(type_name = "TEXT", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Replication {
    Physical,
    Synthetic,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::Type, Serialize, specta::Type)]
#[sqlx(type_name = "TEXT", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderType {
    Market,
    Limit,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::Type, Serialize, specta::Type)]
#[sqlx(type_name = "TEXT", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::Type, Serialize, specta::Type)]
#[sqlx(type_name = "TEXT", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TradeFeeType {
    Broker,
    Tob,
    Fx,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::Type, Serialize, specta::Type)]
#[sqlx(type_name = "TEXT", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DividendFeeType {
    WithholdingTax,
    BrokerFee,
    Fx,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::Type, Serialize, specta::Type)]
#[sqlx(type_name = "TEXT", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CashDirection {
    Credit,
    Debit,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::Type, Serialize, specta::Type)]
#[sqlx(type_name = "TEXT", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CashCategory {
    Trade,
    Fee,
    Tax,
    Dividend,
    Transfer,
    Fx,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::Type, Serialize, specta::Type)]
#[sqlx(type_name = "TEXT", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CorporateActionType {
    Split,
    ReverseSplit,
    MergerInto,
    SpinOff,
    SymbolChange,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::Type, Serialize, specta::Type)]
#[sqlx(type_name = "TEXT", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FxRateSource {
    Ecb,
    Nbb,
    Broker,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::Type, Serialize, specta::Type)]
#[sqlx(type_name = "TEXT", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PriceSource {
    Exchange,
    Broker,
    Manual,
}

// ============================================================
// Row structs
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow, Serialize, specta::Type)]
#[sqlx(type_name = "TEXT", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct Currency {
    pub code: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct FundFamily {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Broker {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CgtParameters {
    pub tax_year: i64,
    pub exemption_base_eur: String,
    pub exemption_cap_eur: String,
    pub carryforward_cap_eur: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Instrument {
    pub id: i64,
    pub isin: String,
    pub name: String,
    pub issuer: Option<String>,
    pub instrument_type: InstrumentType,
    pub replication: Option<Replication>,
    pub fsma_registered: bool,
    pub fund_family_id: Option<i64>,
    pub accumulating: bool,
    pub domicile: Option<String>,
    pub subject_to_cgt: bool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Cash {
    pub id: i64,
    pub broker_id: i64,
    pub currency_code: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct FxRate {
    pub id: i64,
    pub date: NaiveDate,
    pub currency: String,
    pub rate_to_eur: String,
    pub source: FxRateSource,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CgtExemptionUsage {
    pub tax_year: i64,
    pub carryforward_in_eur: String,
    pub used_eur: Option<String>,
    pub carryforward_out_eur: Option<String>,
    pub is_confirmed: bool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Listing {
    pub id: i64,
    pub instrument_id: i64,
    pub exchange_mic: String,
    pub ticker: String,
    pub currency_code: String,
    pub settlement_currency_code: Option<String>,
    pub delisted_at: Option<NaiveDate>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BrokerOrder {
    pub id: i64,
    pub listing_id: i64,
    pub broker_id: i64,
    pub order_type: OrderType,
    pub side: Side,
    pub requested_quantity: String,
    pub limit_price: Option<String>,
    pub placed_at: DateTime<Utc>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub broker_order_ref: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Dividend {
    pub id: i64,
    pub instrument_id: i64,
    pub broker_id: i64,
    pub ex_date: NaiveDate,
    pub pay_date: NaiveDate,
    pub quantity_held: String,
    pub gross_per_unit: String,
    pub currency_code: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Trade {
    pub id: i64,
    pub broker_order_id: Option<i64>,
    pub listing_id: i64,
    pub broker_id: i64,
    pub side: Side,
    pub quantity: String,
    pub price: String,
    pub executed_at: DateTime<Utc>,
    pub settlement_cash_id: Option<i64>,
    pub settlement_date: Option<NaiveDate>,
    pub broker_trade_ref: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TradeFee {
    pub id: i64,
    pub trade_id: i64,
    pub fee_type: TradeFeeType,
    pub amount: String,
    pub currency_code: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CorporateAction {
    pub id: i64,
    pub instrument_id: i64,
    pub action_type: CorporateActionType,
    pub effective_date: NaiveDate,
    pub ratio_from: String,
    pub ratio_to: String,
    pub cost_basis_factor: Option<String>,
    pub target_instrument_id: Option<i64>,
    pub spin_off_ratio: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Lot {
    pub id: i64,
    pub broker_id_at_acquisition: i64,
    pub instrument_id: i64,
    pub listing_id: i64,
    pub source_trade_id: Option<i64>,
    pub source_ca_id: Option<i64>,
    pub parent_lot_id: Option<i64>,
    pub qty_at_acquisition: String,
    pub price_currency_code: String,
    pub price_per_unit: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CashTransaction {
    pub id: i64,
    pub cash_id: i64,
    pub direction: CashDirection,
    pub amount: String,
    pub transacted_at: DateTime<Utc>,
    pub category: CashCategory,
    pub trade_id: Option<i64>,
    pub dividend_id: Option<i64>,
    pub source_ca_id: Option<i64>,
    pub description: Option<String>,
    pub fee_fx_conversion_id: Option<i64>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct FxConversion {
    pub id: i64,
    pub debit_cash_transaction_id: i64,
    pub credit_cash_transaction_id: i64,
    pub rate: String,
    pub executed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SellAllocation {
    pub id: i64,
    pub sell_trade_id: i64,
    pub origin_lot_id: i64,
    pub quantity: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LotClose {
    pub id: i64,
    pub lot_id: i64,
    pub source_ca_id: i64,
    pub closed_at: DateTime<Utc>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TaxSnapshot2025 {
    pub id: i64,
    pub instrument_id: i64,
    pub broker_id: i64,
    pub qty_at_snapshot: String,
    pub hist_cost_per_unit_eur: String,
    pub snap_price_per_unit_eur: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TaxSellAllocation {
    pub id: i64,
    pub sell_allocation_id: i64,
    pub sale_price_eur: String,
    pub buy_price_eur: String,
    pub taxable_gain_eur: String,
    pub tax_year: i64,
    pub computed_at: DateTime<Utc>,
    pub sale_fx_rate_id: Option<i64>,
    pub buy_fx_rate_id: Option<i64>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DividendFee {
    pub id: i64,
    pub dividend_id: i64,
    pub fee_type: DividendFeeType,
    pub amount: String,
    pub currency_code: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LotTransfer {
    pub id: i64,
    pub lot_id: i64,
    pub from_broker_id: i64,
    pub to_broker_id: i64,
    pub transferred_at: DateTime<Utc>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PriceHistory {
    pub id: i64,
    pub listing_id: i64,
    pub date: NaiveDate,
    pub close: String,
    pub source: PriceSource,
}
