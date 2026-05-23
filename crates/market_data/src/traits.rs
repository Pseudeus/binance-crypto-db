use std::time::{SystemTime, UNIX_EPOCH};

pub trait RemoteResponse<T> {
    fn to_insertable(&self) -> Result<T, serde_json::Error>;

    fn get_time_f64(&self) -> f64 {
        let now = SystemTime::now();
        let timestamp_float = now
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);

        timestamp_float
    }
}
