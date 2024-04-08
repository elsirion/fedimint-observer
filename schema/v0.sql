CREATE TABLE IF NOT EXISTS federations (
    federation_id BLOB PRIMARY KEY NOT NULL,
    config BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS sessions (
    federation_id BLOB NOT NULL REFERENCES federations(federation_id),
    session_index INTEGER NOT NULL,
    -- TODO: add transaction and item count
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

CREATE TABLE IF NOT EXISTS transaction_inputs (
    federation_id BLOB NOT NULL REFERENCES federations(federation_id),
    txid BLOB NOT NULL,
    in_index INTEGER NOT NULL,
    kind TEXT NOT NULL,
    ln_contract_id BLOB,
    amount_msat INTEGER,
    PRIMARY KEY (federation_id, txid, in_index),
    FOREIGN KEY (federation_id, txid) REFERENCES transactions(federation_id, txid), -- This might be a bit too heavy of a foreign key? Maybe use rowid instead?
    FOREIGN KEY (federation_id, ln_contract_id) REFERENCES ln_contracts(federation_id, contract_id)
);
CREATE INDEX IF NOT EXISTS federation_inputs ON transaction_inputs(federation_id);
CREATE INDEX IF NOT EXISTS federation_transaction_inputs ON transaction_inputs(federation_id, txid);
CREATE INDEX IF NOT EXISTS federation_input_kinds ON transaction_inputs(federation_id, kind);

CREATE TABLE IF NOT EXISTS transaction_outputs (
    federation_id BLOB NOT NULL REFERENCES federations(federation_id),
    txid BLOB NOT NULL,
    out_index INTEGER NOT NULL,
    kind TEXT NOT NULL,
    -- We keep the ln contract relation denormalized for now. If additional modules need extra data attached to
    -- inputs/outputs we'll have to refactor that or introduce some constraints to keep the complexity manageable.
    ln_contract_interaction_kind TEXT CHECK (ln_contract_interaction_kind IN ('fund', 'cancel', 'offer', NULL)),
    ln_contract_id BLOB,
    amount_msat INTEGER,
    PRIMARY KEY (federation_id, txid, out_index),
    FOREIGN KEY (federation_id, txid) REFERENCES transactions(federation_id, txid) -- This might be a bit too heavy of a foreign key? Maybe use rowid instead?
    -- Can't apply the following FK constraint because contract doesn't exist yet when offers are created:
    -- FOREIGN KEY (federation_id, ln_contract_id) REFERENCES ln_contracts(federation_id, contract_id)
);
CREATE INDEX IF NOT EXISTS federation_outputs ON transaction_outputs(federation_id);
CREATE INDEX IF NOT EXISTS federation_transaction_outputs ON transaction_outputs(federation_id, txid);
CREATE INDEX IF NOT EXISTS federation_output_kinds ON transaction_outputs(federation_id, kind);

CREATE TABLE IF NOT EXISTS ln_contracts (
    federation_id BLOB NOT NULL REFERENCES federations(federation_id),
    contract_id BLOB NOT NULL,
    type TEXT NOT NULL CHECK (type IN ('incoming', 'outgoing')),
    payment_hash BLOB NOT NULL,
    PRIMARY KEY (federation_id, contract_id)
);
CREATE INDEX IF NOT EXISTS ln_contract_federation_contract ON ln_contracts(federation_id, contract_id);
CREATE INDEX IF NOT EXISTS ln_contract_federation ON ln_contracts (federation_id);
CREATE INDEX IF NOT EXISTS ln_contract_hashes ON ln_contracts (payment_hash);
CREATE INDEX IF NOT EXISTS ln_contract_gateways ON ln_contracts(gateway_id);

CREATE TABLE IF NOT EXISTS block_times (
    block_height INTEGER PRIMARY KEY,
    timestamp INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS block_times_time ON block_times(timestamp);
