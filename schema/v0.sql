CREATE TABLE IF NOT EXISTS federations (
    federation_id BLOB PRIMARY KEY NOT NULL,
    config BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS sessions (
    federation_id BLOB NOT NULL REFERENCES federations(federation_id),
    session_index INTEGER NOT NULL,
    session BLOB NOT NULL,
    PRIMARY KEY (federation_id, session_index)
);
CREATE INDEX IF NOT EXISTS federation_sessions ON sessions(federation_id);

CREATE TABLE IF NOT EXISTS transactions (
    txid BLOB NOT NULL,
    federation_id BLOB NOT NULL REFERENCES federations(federation_id),
    session_index INTEGER NOT NULL,
    item_index INTEGER NOT NULL,
    data BLOB NOT NULL,
    FOREIGN KEY (federation_id, session_index) REFERENCES sessions(federation_id, session_index),
    PRIMARY KEY (federation_id, txid)
);
CREATE INDEX IF NOT EXISTS federation_transactions ON transactions(federation_id);
CREATE INDEX IF NOT EXISTS federation_session_transactions ON transactions(federation_id, session_index);
