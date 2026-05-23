CREATE TABLE IF NOT EXISTS order_books(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    time REAL NOT NULL,
    symbol_id TEXT NOT NULL,
    bids BLOB NOT NULL,
    asks BLOB NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_time ON order_books(time);
CREATE INDEX IF NOT EXISTS idx_symbol_time ON order_books(symbol_id, time);

CREATE TABLE IF NOT EXISTS agg_trades(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    time REAL NOT NULL,
    symbol_id TEXT NOT NULL,
    price REAL NOT NULL,
    quantity REAL NOT NULL,
    is_buyer_maker BOOLEAN NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_agg_symbol_time ON agg_trades(symbol_id, time);

CREATE TABLE IF NOT EXISTS klines_1s(
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
CREATE INDEX IF NOT EXISTS idx_klines_symbol_starttime ON klines_1s(symbol_id, start_time);

-- NOT YET BEING USED
CREATE TABLE IF NOT EXISTS funding_rates(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    time REAL NOT NULL,
    symbol_id TEXT NOT NULL,
    mark_price REAL NOT NULL,
    index_price REAL NOT NULL,
    rate REAL NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_funding_rates_symbol_time ON funding_rates(symbol_id, time);

CREATE TABLE IF NOT EXISTS open_interest(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    time REAL NOT NULL,
    symbol_id TEXT NOT NULL,
    oi_value REAL NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_open_interest_symbol_time ON open_interest(symbol_id, time);

-- NOT YET BEING USED
CREATE TABLE IF NOT EXISTS liquidations(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    time REAL NOT NULL,
    symbol_id TEXT NOT NULL,
    side TEXT NOT NULL,
    price REAL NOT NULL,
    quantity REAL NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_liquidations_symbol_time ON liquidations(symbol_id, time);
