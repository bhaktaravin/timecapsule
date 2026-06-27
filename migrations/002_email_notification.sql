ALTER TABLE capsules ADD COLUMN encrypted_unlock_token BLOB;
ALTER TABLE capsules ADD COLUMN unlock_token_nonce BLOB;
ALTER TABLE capsules ADD COLUMN notification_sent_at TEXT;
