use common::models::OpenInterestInsert;
use serde::Deserialize;

use crate::traits::RemoteResponse;

#[derive(Debug, Deserialize)]
pub struct OpenInterestResponse {
    pub symbol: String,
    #[serde(rename(deserialize = "openInterest"))]
    pub open_interest: String,
}

impl RemoteResponse<OpenInterestInsert> for OpenInterestResponse {
    fn to_insertable(&self) -> Result<OpenInterestInsert, serde_json::Error> {
        Ok(OpenInterestInsert {
            time: self.get_time_f64(),
            symbol: self.symbol.clone(),
            oi_value: self.open_interest.parse::<f64>().unwrap_or(0_f64),
        })
    }
}
