ALTER TABLE virtual_keys
    DROP COLUMN encrypted_key,
    DROP COLUMN key_nonce,
    DROP COLUMN key_version;
