use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub time: String,
    pub kind: String,
    pub text: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GraphLink {
    pub from: String,
    pub to: String,
    pub relation: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    pub time: String,
    pub text: String,
    pub source: String,
    pub importance: f32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LearningFact {
    pub subject: String,
    pub subject_label: String,
    pub subject_kind: String,
    pub relation: String,
    pub object: String,
    pub object_label: String,
    pub object_kind: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LearningResponse {
    pub facts: Vec<LearningFact>,
}

#[derive(Clone, Debug, Default)]
pub struct TokenMarket {
    pub name: String,
    pub mint: String,
    pub price: Option<f64>,
    pub price_change_24h: Option<f64>,
    pub volume_24h: Option<f64>,
    pub liquidity: Option<f64>,
    pub market_cap: Option<f64>,
    pub candles: Vec<(i64, f64)>,
    pub pool: Option<String>,
}
