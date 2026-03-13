use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Capability {
    #[serde(rename = "ohlcv")]
    Ohlcv,
    #[serde(rename = "fundamentals")]
    Fundamentals,
    #[serde(rename = "reference")]
    Reference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Market {
    #[serde(rename = "us_equity")]
    UsEquity,
    #[serde(rename = "crypto")]
    Crypto,
    #[serde(rename = "cn_equity")]
    CnEquity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawData {
    pub content: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedData {
    pub records: Vec<serde_json::Value>,
    pub metadata: serde_json::Value,
}
