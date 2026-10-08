#[derive(Clone, Debug, PartialEq)]
pub struct OracleValue {
    pub feed: String,
    pub value: f64,
    pub confidence: Option<f64>,
    pub timestamp_ms: i64,
    pub source: String,
}

impl OracleValue {
    pub fn validate(&self) -> Result<(), String> {
        if self.feed.trim().is_empty() || self.source.trim().is_empty() {
            return Err("oracle feed/source is empty".into());
        }
        if !self.value.is_finite() { return Err("oracle value is not finite".into()); }
        if let Some(confidence) = self.confidence {
            if !(0.0..=1.0).contains(&confidence) {
                return Err("oracle confidence must be between 0 and 1".into());
            }
        }
        Ok(())
    }
}

pub const SWITCHBOARD_CRATE: &str = "switchboard-on-demand";
