CREATE TABLE IF NOT EXISTS connections (
    id INTEGER PRIMARY KEY,
    uid TEXT NOT NULL,
    last_seen INTEGER NOT NULL
);
CREATE UNIQUE INDEX idx_connection_uid ON connections(uid); 

CREATE TABLE IF NOT EXISTS received_messages (
    id INTEGER PRIMARY KEY,
    connection_id INTEGER NOT NULL,
    uid TEXT NOT NULL,
    data REAL NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY(connection_id) REFERENCES connections(id)
);

CREATE TABLE IF NOT EXISTS queued_messages (
    id INTEGER PRIMARY KEY,
    message TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS delivered_messages (
    id INTEGER PRIMARY KEY,
    uid TEXT NOT NULL,
    queued_message_id INTEGER NOT NULL,
    connection_id INTEGER NOT NULL
    FOREIGN KEY(queued_message_id) REFERENCES queued_messages(id)
    FOREIGN KEY(connection_id) REFERENCES connections(id)
);
CREATE UNIQUE INDEX idx_delivered_uid ON delivered_messages(uid); 
