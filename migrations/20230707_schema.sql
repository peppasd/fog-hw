CREATE TABLE IF NOT EXISTS connections (
    id INTEGER PRIMARY KEY,
    uid TEXT NOT NULL,
    last_seen INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS sent_messages (
    id INTEGER PRIMARY KEY,
    uid TEXT NOT NULL,
    data REAL NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS queued_messages (
    id INTEGER PRIMARY KEY,
    message TEXT NOT NULL,
    created_at INTEGER NOT NULL
);
