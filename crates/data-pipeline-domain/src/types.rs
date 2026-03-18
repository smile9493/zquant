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

impl std::fmt::Display for Capability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Capability::Ohlcv => write!(f, "ohlcv"),
            Capability::Fundamentals => write!(f, "fundamentals"),
            Capability::Reference => write!(f, "reference"),
        }
    }
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

impl std::fmt::Display for Market {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Market::UsEquity => write!(f, "us_equity"),
            Market::Crypto => write!(f, "crypto"),
            Market::CnEquity => write!(f, "cn_equity"),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_display_wire_values() {
        assert_eq!(Capability::Ohlcv.to_string(), "ohlcv");
        assert_eq!(Capability::Fundamentals.to_string(), "fundamentals");
        assert_eq!(Capability::Reference.to_string(), "reference");
    }

    #[test]
    fn market_display_wire_values() {
        assert_eq!(Market::UsEquity.to_string(), "us_equity");
        assert_eq!(Market::Crypto.to_string(), "crypto");
        assert_eq!(Market::CnEquity.to_string(), "cn_equity");
    }
}
