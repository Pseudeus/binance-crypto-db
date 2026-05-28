use std::time::{SystemTime, UNIX_EPOCH};

pub trait RemoteResponse<T> {
    fn to_insertable(&self) -> Result<T, serde_json::Error>;

    fn get_time_i64(&self) -> i64 {
        let now = SystemTime::now();
        let timestamp_i64 = now
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0_i64);

        timestamp_i64
    }
}
