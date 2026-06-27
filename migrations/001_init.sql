CREATE TABLE capsules (
    id TEXT PRIMARY KEY NOT NULL,
    recipient_email TEXT NOT NULL,
    unlock_at TEXT NOT NULL,
    encrypted_payload BLOB NOT NULL,
    payload_nonce BLOB NOT NULL,
    wrapped_dek BLOB NOT NULL,
    wrapped_dek_nonce BLOB NOT NULL,
    unlock_token_hash BLOB NOT NULL,
    status TEXT NOT NULL DEFAULT 'sealed',
    created_at TEXT NOT NULL,
    opened_at TEXT
);

CREATE INDEX idx_capsules_unlock_at ON capsules(unlock_at);
CREATE INDEX idx_capsules_status ON capsules(status);
