-- Lightning Gateway Registrations: Normalized storage for gateway announcements
BEGIN;

INSERT INTO schema_version (version) VALUES (9);

-- Normalized table for gateway registrations
CREATE TABLE IF NOT EXISTS ln_gateway_registrations (
    -- Identity
    federation_id BYTEA NOT NULL REFERENCES federations (federation_id),
    gateway_id BYTEA NOT NULL,
    
    -- Source tracking (for debugging/auditing)
    session_index INTEGER NOT NULL,
    item_index INTEGER NOT NULL,
    proposer INTEGER NOT NULL,
    registered_at TIMESTAMP NOT NULL,
    
    -- Gateway info
    node_pub_key BYTEA NOT NULL,
    api_endpoint TEXT NOT NULL,
    
    -- Fee structure
    base_fee_msat BIGINT NOT NULL,
    proportional_fee_millionths INTEGER NOT NULL,
    
    -- Capabilities
    supports_private_payments BOOLEAN NOT NULL DEFAULT false,
    
    -- Expiry
    ttl_seconds INTEGER NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    
    -- Raw data (for completeness)
    route_hints JSONB,
    
    PRIMARY KEY (federation_id, gateway_id, session_index, item_index),
    FOREIGN KEY (federation_id, session_index, item_index) 
        REFERENCES consensus_items (federation_id, session_index, item_index)
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS ln_gateway_active ON ln_gateway_registrations (
    federation_id,
    gateway_id,
    expires_at DESC
);

CREATE INDEX IF NOT EXISTS ln_gateway_by_fed ON ln_gateway_registrations (
    federation_id, 
    expires_at DESC
);

CREATE INDEX IF NOT EXISTS ln_gateway_by_session ON ln_gateway_registrations (
    federation_id,
    session_index,
    item_index
);

-- Current active gateways (deduplicated, non-expired)
CREATE MATERIALIZED VIEW ln_current_gateways AS
WITH latest_registrations AS (
    SELECT 
        federation_id,
        gateway_id,
        node_pub_key,
        api_endpoint,
        base_fee_msat,
        proportional_fee_millionths,
        supports_private_payments,
        registered_at,
        expires_at,
        route_hints,
        ROW_NUMBER() OVER (
            PARTITION BY federation_id, gateway_id 
            ORDER BY session_index DESC, item_index DESC
        ) as rn
    FROM ln_gateway_registrations
    WHERE expires_at > NOW()
)
SELECT 
    federation_id,
    gateway_id,
    node_pub_key,
    api_endpoint,
    base_fee_msat,
    proportional_fee_millionths,
    supports_private_payments,
    registered_at,
    expires_at,
    route_hints,
    EXTRACT(EPOCH FROM (expires_at - NOW()))::INTEGER as seconds_until_expiry
FROM latest_registrations
WHERE rn = 1;

-- Indexes on the materialized view
CREATE INDEX IF NOT EXISTS ln_current_gateways_fed ON ln_current_gateways (federation_id);
CREATE INDEX IF NOT EXISTS ln_current_gateways_gateway ON ln_current_gateways (gateway_id);
CREATE INDEX IF NOT EXISTS ln_current_gateways_fees ON ln_current_gateways (
    federation_id,
    base_fee_msat,
    proportional_fee_millionths
);-- Lightning Gateway Registrations: Normalized storage for gateway announcements
BEGIN;

INSERT INTO schema_version (version) VALUES (9);

-- Normalized table for gateway registrations
CREATE TABLE IF NOT EXISTS ln_gateway_registrations (
    -- Identity
    federation_id BYTEA NOT NULL REFERENCES federations (federation_id),
    gateway_id BYTEA NOT NULL,
    
    -- Source tracking (for debugging/auditing)
    session_index INTEGER NOT NULL,
    item_index INTEGER NOT NULL,
    proposer INTEGER NOT NULL,
    registered_at TIMESTAMP NOT NULL,
    
    -- Gateway info
    node_pub_key BYTEA NOT NULL,
    api_endpoint TEXT NOT NULL,
    
    -- Fee structure
    base_fee_msat BIGINT NOT NULL,
    proportional_fee_millionths INTEGER NOT NULL,
    
    -- Capabilities
    supports_private_payments BOOLEAN NOT NULL DEFAULT false,
    
    -- Expiry
    ttl_seconds INTEGER NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    
    -- Raw data (for completeness)
    route_hints JSONB,
    
    PRIMARY KEY (federation_id, gateway_id, session_index, item_index),
    FOREIGN KEY (federation_id, session_index, item_index) 
        REFERENCES consensus_items (federation_id, session_index, item_index)
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS ln_gateway_active ON ln_gateway_registrations (
    federation_id,
    gateway_id,
    expires_at DESC
);

CREATE INDEX IF NOT EXISTS ln_gateway_by_fed ON ln_gateway_registrations (
    federation_id, 
    expires_at DESC
);

CREATE INDEX IF NOT EXISTS ln_gateway_by_session ON ln_gateway_registrations (
    federation_id,
    session_index,
    item_index
);

-- Current active gateways (deduplicated, non-expired)
CREATE MATERIALIZED VIEW ln_current_gateways AS
WITH latest_registrations AS (
    SELECT 
        federation_id,
        gateway_id,
        node_pub_key,
        api_endpoint,
        base_fee_msat,
        proportional_fee_millionths,
        supports_private_payments,
        registered_at,
        expires_at,
        route_hints,
        ROW_NUMBER() OVER (
            PARTITION BY federation_id, gateway_id 
            ORDER BY session_index DESC, item_index DESC
        ) as rn
    FROM ln_gateway_registrations
    WHERE expires_at > NOW()
)
SELECT 
    federation_id,
    gateway_id,
    node_pub_key,
    api_endpoint,
    base_fee_msat,
    proportional_fee_millionths,
    supports_private_payments,
    registered_at,
    expires_at,
    route_hints,
    EXTRACT(EPOCH FROM (expires_at - NOW()))::INTEGER as seconds_until_expiry
FROM latest_registrations
WHERE rn = 1;

-- Indexes on the materialized view
CREATE INDEX IF NOT EXISTS ln_current_gateways_fed ON ln_current_gateways (federation_id);
CREATE INDEX IF NOT EXISTS ln_current_gateways_gateway ON ln_current_gateways (gateway_id);
CREATE INDEX IF NOT EXISTS ln_current_gateways_fees ON ln_current_gateways (
    federation_id,
    base_fee_msat,
    proportional_fee_millionths
);

-- Unique index required to allow CONCURRENTLY refresh of the materialized view
CREATE UNIQUE INDEX IF NOT EXISTS ln_current_gateways_unique ON ln_current_gateways (
    federation_id,
    gateway_id
);

COMMIT;

-- Unique index required to allow CONCURRENTLY refresh of the materialized view
CREATE UNIQUE INDEX IF NOT EXISTS ln_current_gateways_unique ON ln_current_gateways (
    federation_id,
    gateway_id
);

COMMIT;