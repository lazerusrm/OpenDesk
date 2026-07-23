CREATE TABLE migration_legacy_credentials (
    user_uuid TEXT PRIMARY KEY NOT NULL REFERENCES users(user_uuid) ON DELETE CASCADE,
    run_id TEXT NOT NULL REFERENCES migration_runs(run_id) ON DELETE RESTRICT,
    verifier_algorithm TEXT NOT NULL CHECK (verifier_algorithm = 'bcrypt'),
    verifier TEXT NOT NULL CHECK (
        length(verifier) = 60
        AND substr(verifier, 1, 7) = '$2b$06$'
    ),
    created_at TEXT NOT NULL,
    consumed_at TEXT
);

CREATE INDEX idx_migration_legacy_credentials_run_id
    ON migration_legacy_credentials(run_id);
