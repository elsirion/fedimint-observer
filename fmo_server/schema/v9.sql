BEGIN;

INSERT INTO
    schema_version (version)
VALUES
    (9);

CREATE TABLE IF NOT EXISTS gateways (
    federation_id   BYTEA        NOT NULL REFERENCES federations (federation_id),
    gateway_id      TEXT         NOT NULL,
    node_pub_key    TEXT         NOT NULL,
    api_endpoint    TEXT         NOT NULL,
    lightning_alias TEXT         NOT NULL,
    vetted          BOOLEAN      NOT NULL DEFAULT FALSE,
    raw             JSONB        NOT NULL,
    first_seen      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    last_seen       TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    PRIMARY KEY (federation_id, gateway_id)
);

CREATE INDEX IF NOT EXISTS gateways_federation_id ON gateways (federation_id);
CREATE INDEX IF NOT EXISTS gateways_node_pub_key  ON gateways (node_pub_key);

COMMIT;
