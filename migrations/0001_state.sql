-- Crash-Safe SQLite WAL Configuration
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;

-- 1. Orders Table: Tracks all order intents and matching engine statuses
CREATE TABLE IF NOT EXISTS orders (
    client_order_id TEXT PRIMARY KEY,
    venue TEXT NOT NULL,
    symbol TEXT NOT NULL,
    side TEXT NOT NULL,
    order_type TEXT NOT NULL,
    price TEXT,
    qty TEXT NOT NULL,
    stop_loss_price TEXT,
    take_profit_price TEXT,
    status TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

-- 2. Fills Table: Individual execution fills with fee tracking
CREATE TABLE IF NOT EXISTS fills (
    fill_id TEXT PRIMARY KEY,
    client_order_id TEXT NOT NULL,
    symbol TEXT NOT NULL,
    side TEXT NOT NULL,
    fill_price TEXT NOT NULL,
    fill_qty TEXT NOT NULL,
    fee_qty TEXT NOT NULL,
    fee_currency TEXT NOT NULL,
    filled_at_ms INTEGER NOT NULL,
    FOREIGN KEY (client_order_id) REFERENCES orders(client_order_id)
);

-- 3. Positions Table: Active spot inventory, current stop and ratchet stage
CREATE TABLE IF NOT EXISTS positions (
    symbol TEXT PRIMARY KEY,
    venue TEXT NOT NULL,
    qty TEXT NOT NULL,
    entry_price TEXT NOT NULL,
    current_stop_price TEXT NOT NULL,
    breakeven_active INTEGER NOT NULL DEFAULT 0,
    highest_price TEXT NOT NULL,
    bars_held INTEGER NOT NULL DEFAULT 0,
    updated_at_ms INTEGER NOT NULL
);

-- 4. Circuit Breaker & Halt State: Singleton table for emergency halts
CREATE TABLE IF NOT EXISTS halt_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    is_halted INTEGER NOT NULL DEFAULT 0,
    halt_reason TEXT,
    halted_at_ms INTEGER
);

-- 5. Equity High-Water Mark: Tracks peak equity to enforce -6.0% drawdown limit
CREATE TABLE IF NOT EXISTS equity_hwm (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    high_water_mark TEXT NOT NULL,
    current_equity TEXT NOT NULL,
    max_drawdown_pct TEXT NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

-- Seed singleton initial states
INSERT OR IGNORE INTO halt_state (id, is_halted, halt_reason, halted_at_ms)
VALUES (1, 0, NULL, NULL);

INSERT OR IGNORE INTO equity_hwm (id, high_water_mark, current_equity, max_drawdown_pct, updated_at_ms)
VALUES (1, '100.0', '100.0', '0.0', 0);

-- Indices for fast query lookups during execution loops
CREATE INDEX IF NOT EXISTS idx_orders_status ON orders(status);
CREATE INDEX IF NOT EXISTS idx_fills_client_order_id ON fills(client_order_id);
