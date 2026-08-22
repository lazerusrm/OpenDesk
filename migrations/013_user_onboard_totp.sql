CREATE TABLE user_onboard_totp (
    user_uuid TEXT PRIMARY KEY NOT NULL REFERENCES users(user_uuid) ON DELETE CASCADE,
    secret_hex TEXT NOT NULL,
    created_at TEXT NOT NULL,
    CHECK (length(secret_hex) = 40)
);
