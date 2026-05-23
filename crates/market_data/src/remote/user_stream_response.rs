use serde::Deserialize;
use common::models::{AccountUpdate, ExecutionReport, Price, Quantity};
use crate::traits::RemoteResponse;

#[derive(Deserialize, Debug)]
pub struct AccountPositionEvent {
    #[serde(rename = "B")]
    pub balances: Vec<AssetBalance>,
}

#[derive(Deserialize, Debug)]
pub struct AssetBalance {
    #[serde(rename = "a")]
    pub asset: String,
    #[serde(rename = "f")]
    pub free: String,
    #[serde(rename = "l")]
    pub locked: String,
}

#[derive(Deserialize, Debug)]
pub struct ExecutionReportEvent {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "S")]
    pub side: String,
    #[serde(rename = "p")]
    pub price: String,
    #[serde(rename = "q")]
    pub quantity: String,
    #[serde(rename = "X")]
    pub status: String,
    #[serde(rename = "n")]
    pub commission: String,
    #[serde(rename = "N")]
    pub commission_asset: Option<String>,
}

impl RemoteResponse<Vec<AccountUpdate>> for AccountPositionEvent {
    fn to_insertable(&self) -> Result<Vec<AccountUpdate>, serde_json::Error> {
        Ok(self.balances.iter().map(|b| AccountUpdate {
            asset: b.asset.clone(),
            free: b.free.parse::<f64>().unwrap_or(0.0),
            locked: b.locked.parse::<f64>().unwrap_or(0.0),
        }).collect())
    }
}

impl RemoteResponse<ExecutionReport> for ExecutionReportEvent {
    fn to_insertable(&self) -> Result<ExecutionReport, serde_json::Error> {
        Ok(ExecutionReport {
            symbol: self.symbol.clone(),
            side: self.side.clone(),
            price: Price(self.price.parse::<f64>().unwrap_or(0.0)),
            quantity: Quantity(self.quantity.parse::<f64>().unwrap_or(0.0)),
            status: self.status.clone(),
            commission: self.commission.parse::<f64>().unwrap_or(0.0),
            commission_asset: self.commission_asset.clone().unwrap_or_default(),
        })
    }
}
