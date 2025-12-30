use hmac::{Hmac, Mac};
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{error, info};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Serialize)]
pub struct OrderRequest {
    pub symbol: String,
    pub side: String, // BUY or SELL
    #[serde(rename = "type")]
    pub order_type: String, // LIMIT, MARKET, etc.
    #[serde(rename = "timeInForce")]
    pub time_in_force: Option<String>, // GTC, IOC, etc.
    pub quantity: f64,
    pub price: Option<f64>,
    pub timestamp: u64,
}

#[derive(Debug, Deserialize)]
pub struct OrderResponse {
    #[serde(rename = "orderId")]
    pub order_id: u64,
    pub symbol: String,
    pub status: String,
    #[serde(rename = "executedQty")]
    pub executed_qty: String,
    #[serde(rename = "cummulativeQuoteQty")]
    pub cummulative_quote_qty: String,
}

#[derive(Debug, Deserialize)]
pub struct Balance {
    pub asset: String,
    pub free: String,
    pub locked: String,
}

#[derive(Debug, Deserialize)]
pub struct AccountInformation {
    pub balances: Vec<Balance>,
    #[serde(rename = "canTrade")]
    pub can_trade: bool,
}

#[derive(Clone)]
pub struct BinanceClient {
    client: Client,
    base_url: String,
    api_key: String,
    secret_key: String,
}

impl BinanceClient {
    pub fn new() -> Self {
        let api_key = env::var("BINANCE_API_KEY").expect("BINANCE_API_KEY not set");
        let secret_key = env::var("BINANCE_SECRET_KEY").expect("BINANCE_SECRET_KEY not set");
        let base_url = env::var("BINANCE_BASE_URL").unwrap_or_else(|_| "https://api.binance.com".to_string());

        Self {
            client: Client::new(),
            base_url,
            api_key,
            secret_key,
        }
    }

    fn sign(&self, query: &str) -> String {
        let mut mac = HmacSha256::new_from_slice(self.secret_key.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(query.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    pub async fn get_account(&self) -> Result<AccountInformation, Box<dyn std::error::Error + Send + Sync>> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
            
        let params = format!("timestamp={}", timestamp);
        let signature = self.sign(&params);
        let full_query = format!("{}&signature={}", params, signature);
        let url = format!("{}/api/v3/account?{}", self.base_url, full_query);
        
        let resp = self.client
            .get(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await?;

        if !resp.status().is_success() {
            let error_text = resp.text().await?;
            error!("Binance Account Info Failed: {}", error_text);
            return Err(error_text.into());
        }

        let account_info = resp.json::<AccountInformation>().await?;
        Ok(account_info)
    }

    pub async fn post_order(&self, symbol: &str, side: &str, quantity: f64) -> Result<OrderResponse, Box<dyn std::error::Error + Send + Sync>> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Simple Market Order for MVP
        let params = format!(
            "symbol={}&side={}&type=MARKET&quantity={}&timestamp={}",
            symbol.to_uppercase(),
            side,
            quantity,
            timestamp
        );

        let signature = self.sign(&params);
        let full_query = format!("{}&signature={}", params, signature);
        let url = format!("{}/api/v3/order?{}", self.base_url, full_query);

        info!("Placing Order: {} {} {}", side, quantity, symbol);

        let resp = self
            .client
            .request(Method::POST, &url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await?;

        if !resp.status().is_success() {
            let error_text = resp.text().await?;
            error!("Binance Order Failed: {}", error_text);
            return Err(error_text.into());
        }

        let order_resp = resp.json::<OrderResponse>().await?;
        Ok(order_resp)
    }
}
