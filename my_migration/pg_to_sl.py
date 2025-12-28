import json
import sqlite3
import struct

import psycopg2
from psycopg2.extras import Json

# --- CONFIGURATION ---
PG_DSN = "dbname=crypto_data user=rusted_crab password=** host=192.168.2.226 port=5432"
SQLITE_DB = "/home/auco1120/Documents/Rust/binance-crypto-db/my_migration/crypto_2025_12.db"  # MAKE SURE THIS IS ON YOUR WD BLACK DRIVE
BATCH_SIZE = 20000


# --- HELPER: JSON TO BINARY BLOB ---
def pack_orderbook_level(json_data):
    """
    Converts JSON asks/bids into a flat binary float array for AI training.
    Supports both: [{'p': 100, 'v': 1}, ...] AND [[100, 1], ...]
    """
    # 1. Handle if Psycopg2 didn't auto-convert to list (unlikely but safe)
    data_list = json_data if isinstance(json_data, list) else json.loads(json_data)

    flat_floats = []

    # 2. Extract numbers based on structure
    if not data_list:
        return b""  # Empty blob for empty list

    first_elem = data_list[0]

    # CASE A: List of Dicts (e.g., {"p": 100.5, "v": 10})
    if isinstance(first_elem, dict):
        # Assumes keys 'p' (price) and 'v' (volume) - ADJUST KEYS IF NEEDED
        # If your keys are 'price'/'amount', change them below.
        for item in data_list:
            flat_floats.extend([float(item.get("p", 0)), float(item.get("v", 0))])

    # CASE B: List of Lists (e.g., [100.5, 10])
    elif isinstance(first_elem, (list, tuple)):
        for item in data_list:
            flat_floats.extend([float(item[0]), float(item[1])])

    # 3. Pack into Binary (C-struct style)
    # 'f' = float (4 bytes). Use 'd' for double (8 bytes) if you need extreme precision.
    # For AI, 'f' is usually sufficient and saves 50% space.
    return struct.pack(f"{len(flat_floats)}f", *flat_floats)


# --- MAIN MIGRATION ---
try:
    # 1. Connect to Postgres (Server Side Cursor to save RAM)
    pg_conn = psycopg2.connect(PG_DSN)
    pg_cur = pg_conn.cursor(name="migrator_cursor")  # Named cursor = Server Side
    pg_cur.itersize = BATCH_SIZE

    # 2. Connect to SQLite
    sl_conn = sqlite3.connect(SQLITE_DB)
    sl_cur = sl_conn.cursor()

    # 3. SQLite Optimization (The "Speed Mode")
    sl_conn.execute("PRAGMA journal_mode = WAL;")
    sl_conn.execute("PRAGMA synchronous = OFF;")  # Maximizes SSD write speed
    sl_conn.execute("PRAGMA cache_size = -64000;")  # 64MB Cache
    sl_conn.execute("BEGIN TRANSACTION;")

    # 4. Create Table (Matching your Schema)
    # Note: We do not insert 'id', SQLite auto-generates it.
    sl_cur.execute("""
        CREATE TABLE IF NOT EXISTS order_books (
            id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
            time REAL NOT NULL,
            symbol TEXT NOT NULL,
            bids BLOB NOT NULL,
            asks BLOB NOT NULL
        );
    """)

    # 5. Execute Query on Postgres
    print("Fetching data from PostgreSQL...")
    # Quoting "time" is important because it is a reserved keyword
    pg_cur.execute('SELECT "time", symbol, bids, asks FROM public.order_books')

    count = 0
    print("Starting migration...")

    for row in pg_cur:
        ts_obj, sym, bids_json, asks_json = row

        # A. Process Timestamp
        # Convert Postgres datetime object to Unix Timestamp (Float)
        # This is much faster for AI logic than ISO strings.
        ts_val = ts_obj.timestamp()

        # B. Process JSON -> BLOB
        bids_blob = pack_orderbook_level(bids_json)
        asks_blob = pack_orderbook_level(asks_json)

        # C. Insert into SQLite
        # We explicitly specify columns to skip the 'id' (autoincrement)
        sl_cur.execute(
            "INSERT INTO order_books (time, symbol, bids, asks) VALUES (?, ?, ?, ?)",
            (ts_val, sym, bids_blob, asks_blob),
        )

        count += 1
        if count % BATCH_SIZE == 0:
            print(f"Migrated {count} rows...")

    # 6. Finalize
    print("Committing transaction...")
    sl_conn.commit()

    print("Optimizing database storage (VACUUM)...")
    sl_conn.execute("PRAGMA synchronous = NORMAL;")  # Restore safety
    sl_conn.execute("VACUUM;")

    print(f"Success! Total rows migrated: {count}")

except Exception as e:
    print(f"Error occurred: {e}")
    if "sl_conn" in locals():
        sl_conn.rollback()
finally:
    if "pg_conn" in locals():
        pg_conn.close()
    if "sl_conn" in locals():
        sl_conn.close()
