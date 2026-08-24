CREATE TABLE migration_credential_attachment_receipts (
    run_id TEXT PRIMARY KEY NOT NULL REFERENCES migration_runs(run_id) ON DELETE RESTRICT,
    artifact_sha256 TEXT NOT NULL CHECK (length(artifact_sha256) = 64),
    attached_at TEXT NOT NULL,
    record_count INTEGER NOT NULL CHECK (record_count > 0)
);
