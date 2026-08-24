CREATE TABLE onboard_totp_replay (
    user_uuid TEXT NOT NULL,
    timestep INTEGER NOT NULL,
    used_at TEXT NOT NULL,
    PRIMARY KEY (user_uuid, timestep)
);
