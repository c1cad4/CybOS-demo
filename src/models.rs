pub use cyb_soul::*;

#[derive(Clone, Debug, Default)]
pub struct TokenMarket {
    pub name: String,
    pub price: Option<f64>,
    pub price_change_24h: Option<f64>,
    pub volume_24h: Option<f64>,
    pub liquidity: Option<f64>,
    pub market_cap: Option<f64>,
    pub candles: Vec<(i64, f64)>,
    pub pool: Option<String>,
}
