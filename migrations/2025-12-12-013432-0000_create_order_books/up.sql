-- Your SQL goes here
CREATE TABLE order_books(
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    time REAL NOT NULL,
    symbol TEXT NOT NULL,
    bids BLOB NOT NULL,
    asks BLOB NOT NULL
);

CREATE INDEX idx_time ON order_books(time);
CREATE INDEX idx_symbol ON order_books(symbol);
