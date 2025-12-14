-- Your SQL goes here
CREATE TABLE agg_trades(
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    time REAL NOT NULL,
    symbol TEXT NOT NULL,
    agg_trade_id INTEGER NOT NULL,
    price REAL NOT NULL,
    quantity REAL NOT NULL,
    first_trade_id INTEGER NOT NULL,
    last_trade_id INTEGER NOT NULL,
    is_buyer_maker BOOLEAN NOT NULL
);

CREATE INDEX idx_agg_trades_time ON agg_trades(time);
CREATE INDEX idx_agg_trades_symbol_time ON agg_trades(symbol, time);
CREATE INDEX idx_agg_trades_abb_id ON agg_trades(symbol, agg_trade_id);
