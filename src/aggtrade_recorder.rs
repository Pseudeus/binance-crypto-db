// use std::collections::VecDeque;
// use std::sync::{Arc, Mutex};
// use std::time::Duration;

// use diesel::r2d2::{ConnectionManager, Pool};
// use diesel::{Connection, RunQueryDsl, SqliteConnection};
// use futures_util::StreamExt;
// use tokio::sync::mpsc;
// use tokio::time::Instant;
// use tracing::{debug, error, info, warn};
// use url::Url;

// use crate::models::NewAggTrade;
// use crate::{SYMBOLS, schema};

// const BATCH_SIZE: usize = 200;
// const FLUSH_TIMEOUT_MS: u64 = 2_000;
// const MAX_BUFFER: usize = 2_000;

// type DbPool = Pool<ConnectionManager<SqliteConnection>>;

// #[derive(Debug, thiserror::Error)]
// pub enum AggTradeError {
//     #[error("WebSocket error: {0}")]
//     Ws(#[from] tokio_tungstenite::tungstenite::Error),
//     #[error("JSON error: {0}")]
//     Json(#[from] serde_json::Error),
//     #[error("Database error: {0}")]
//     Db(#[from] diesel::result::Error),
//     #[error("Send error")]
//     Send,
// }

// pub struct AggTradeRecorder {
//     db_pool: DbPool,
//     symbols: Vec<String>,
//     buffer: Arc<Mutex<VecDeque<NewAggTrade>>>,
//     flush_interval: Duration,
// }

// impl AggTradeRecorder {
//     pub fn new(db_pool: DbPool) -> Self {
//         Self {
//             db_pool,
//             symbols: SYMBOLS.iter().map(|&s| s.to_lowercase()).collect(),
//             buffer: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_BUFFER))),
//             flush_interval: Duration::from_millis(FLUSH_TIMEOUT_MS),
//         }
//     }

//     pub async fn start(&self) -> Result<(), AggTradeError> {
//         let streams: Vec<String> = self
//             .symbols
//             .iter()
//             .map(|s| format!("{}@aggTrade", s))
//             .collect();

//         let uri = format!(
//             "wss://stream.binance.com:9443/stream?streams={}",
//             streams.join("/")
//         );

//         let url = Url::parse(&uri).map_err(|e| AggTradeError::Ws(e.into()))?;

//         info!("Connecting to binance aggTrade stream: {}", uri);

//         loop {
//             match tokio_tungstenite::connect_async(&uri).await {
//                 Ok((ws_stream, _)) => {
//                     info!("Connected to Binance aggTrade stream");
//                     let (mut write, mut read) = ws_stream.split();
//                     let buffer = Arc::clone(&self.buffer);
//                     let db_pool = self.db_pool.clone();

//                     let (flush_tx, mut flush_rx) = mpsc::channel::<Vec<NewAggTrade>>(10);
//                     tokio::spawn(Self::db_writer(pool, flush_rx))
//                 }
//                 Err(_) => todo!(),
//             }
//         }
//     }

//     async fn db_writer(pool: DbPool, mut trade_rx: mpsc::Receiver<NewAggTrade>) {
//         let mut buffer = Vec::with_capacity(200);
//         let mut last_flush = Instant::now();

//         loop {
//             tokio::select! {
//                 Some(trade) = trade_rx.recv() => {
//                     buffer.push(trade);
//                     if buffer.len() >= 200 || last_flush.elapsed() >= Duration::from_millis(FLUSH_TIMEOUT_MS) {
//                         Self::
//                     }
//                 }
//             }
//         }
//     }

//     async fn flush_batch(pool: &DbPool, batch: &[NewAggTrade]) {
//         let l_batch = batch.to_vec();
//         let pool = pool.clone();

//         match tokio::task::spawn_blocking(move || {
//             let mut conn = pool.get().expect("Fuck off");

//             conn.transaction::<_, diesel::result::Error, _>(|c| {
//                 diesel::insert_into(schema::agg_trades::table)
//                     .values(&l_batch)
//                     .execute(c)
//                     .map(|_| ())
//             })
//         })
//         .await
//         {
//             Ok(Ok(())) => debug!("Wrote {} aggTrades", batch.len()),
//             Ok(Err(e)) => error!("DB error: {}", e),
//             Err(e) => error!("DB task panic: {}", e),
//         }
//     }
// }
