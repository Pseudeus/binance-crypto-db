CREATE TABLE IF NOT EXISTS spot_order_books(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    receive_time INTEGER NOT NULL,
    symbol_id TEXT NOT NULL,
    bids BLOB NOT NULL,
    asks BLOB NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_symbol_receive_time ON spot_order_books(symbol_id, receive_time);

CREATE TABLE IF NOT EXISTS spot_agg_trades(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    receive_time INTEGER NOT NULL,
    exchange_time INTEGER NOT NULL,
    symbol_id TEXT NOT NULL,
    price REAL NOT NULL,
    quantity REAL NOT NULL,
    is_buyer_maker BOOLEAN NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_agg_symbol_exchange_time ON spot_agg_trades(symbol_id, exchange_time);

CREATE TABLE IF NOT EXISTS spot_klines_1m(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol_id TEXT NOT NULL,
    start_time INTEGER NOT NULL,
    close_time INTEGER NOT NULL,
    open_price REAL NOT NULL,
    close_price REAL NOT NULL,
    high_price REAL NOT NULL,
    low_price REAL NOT NULL,
    volume REAL NOT NULL,
    no_of_trades INTEGER NOT NULL,
    taker_buy_vol REAL NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_klines_symbol_start_time ON spot_klines_1m(symbol_id, start_time);

CREATE TABLE IF NOT EXISTS spot_book_ticker(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    receive_time INTEGER NOT NULL,
    symbol_id TEXT  NOT NULL,
    best_bid_price REAL NOT NULL,
    best_bid_qty REAL NOT NULL,
    best_ask_price REAL NOT NULL,
    best_ask_qty REAL NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_bood_ticker_symbol_receive_time ON spot_book_ticker(symbol_id, receive_time);

CREATE TABLE IF NOT EXISTS fut_liquidations(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    receive_time INTEGER NOT NULL,
    exchange_time INTEGER NOT NULL,
    symbol_id TEXT NOT NULL,
    side TEXT NOT NULL,
    price REAL NOT NULL,
    quantity REAL NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_liquidations_symbol_exchange_time ON fut_liquidations(symbol_id, exchange_time);
