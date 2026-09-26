-- Virtual keys become retrievable by an authorized team member.
--
-- `key_hash` (sha256) stays exactly as it was and remains the ONLY thing
-- authentication looks at — this adds a second, independent representation
-- for retrieval, it does not replace the lookup path.
--
-- Nullable on purpose: keys issued before this migration have no stored
-- ciphertext and can never be revealed. `/reveal` 404s for them rather than
-- pretending otherwise.
ALTER TABLE virtual_keys
    ADD COLUMN encrypted_key BYTEA,
    ADD COLUMN key_nonce     BYTEA,
    ADD COLUMN key_version   SMALLINT;
