use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, bail};
use common::models::OpenInterestInsert;
use reqwest::Client;
use tokio::sync::Semaphore;
use tokio::time::sleep;
use tracing::{debug, warn};

use crate::{remote::openinterest_response::OpenInterestResponse, traits::RemoteResponse};

pub struct BinancePoller {
    client: Client,
    base_url: String,
    semaphore: Arc<Semaphore>,
    request_delay_ms: u64,
}

impl BinancePoller {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("binance_crypto_bot/0.0.1")
                .timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to build HTTP client."),
            base_url: "https://fapi.binance.com".to_string(),
            semaphore: Arc::new(Semaphore::new(5)),
            request_delay_ms: 100,
        }
    }

    pub async fn fetch_all_open_interest(
        &self,
        symbols: &[String],
    ) -> anyhow::Result<Vec<anyhow::Result<OpenInterestInsert>>> {
        let mut results = Vec::with_capacity(symbols.len());

        for (i, symbol) in symbols.iter().enumerate() {
            if i > 0 {
                sleep(Duration::from_millis(self.request_delay_ms)).await;
            }

            let permit = self
                .semaphore
                .clone()
                .acquire_owned()
                .await
                .context("Failed to acquire semaphore permit")?;

            let result = self.fetch_single_open_interest(symbol).await;

            drop(permit);

            if let Err(ref e) = result {
                if Self::is_rate_limit_error(e) {
                    warn!("Rate limit detected, stopping further requests");
                    break;
                }
            }

            results.push(result);
        }

        Ok(results)
    }

    async fn fetch_single_open_interest(&self, symbol: &str) -> anyhow::Result<OpenInterestInsert> {
        let url = format!("{}/fapi/v1/openInterest", self.base_url);

        let mut retry_count = 0;
        let max_retries = 3;

        loop {
            match self.make_request(&url, symbol).await {
                Ok(response) => return Ok(response.to_insertable()?),
                Err(e) => {
                    if Self::is_rate_limit_error(&e) {
                        retry_count += 1;
                        if retry_count > max_retries {
                            bail!("Max retries exceeded for rate limit");
                        }

                        let backoff_seconds = 2_u64.pow(retry_count);
                        warn!(
                            "Rate limited for symbol {}, backing off for {} seconds (attempt {}/{})",
                            symbol, backoff_seconds, retry_count, max_retries
                        );

                        sleep(Duration::from_secs(backoff_seconds)).await;
                        continue;
                    }
                    bail!("Failed to fetch open interest for {}: {}", symbol, e);
                }
            }
        }
    }

    async fn make_request(&self, url: &str, symbol: &str) -> anyhow::Result<OpenInterestResponse> {
        let response = self
            .client
            .get(url)
            .query(&[("symbol", symbol)])
            .send()
            .await
            .context("Failed to send request")?;

        let status = response.status();
        if status == 429 {
            bail!("HTTP 429: Too Many Requests");
        }
        if status == 418 {
            bail!("HTTP 418: IP has been auto-banned");
        }

        if let Some(used_weight) = response.headers().get("x-mbx-used-weight-1m") {
            let used_weight: u32 = used_weight
                .to_str()
                .context("Invalid weight header")?
                .parse()
                .context("Failed to parse weight")?;

            if used_weight > 1000 {
                warn!("High API weight usage: {}", used_weight);
            } else {
                debug!("Used weights: {}/1200", used_weight);
            }
        }

        let data = response
            .json::<OpenInterestResponse>()
            .await
            .context("Failed to parse JSON response")?;
        Ok(data)
    }

    fn is_rate_limit_error(error: &anyhow::Error) -> bool {
        let error_str = error.to_string();
        error_str.contains("429")
            || error_str.contains("418")
            || error_str.contains("Too Mani Requests")
            || error_str.contains("auto-banned")
    }
}

impl Default for BinancePoller {
    fn default() -> Self {
        Self::new()
    }
}
