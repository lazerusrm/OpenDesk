CREATE TABLE users (
    user_uuid TEXT PRIMARY KEY NOT NULL,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'admin',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE sessions (
    session_uuid TEXT PRIMARY KEY NOT NULL,
    user_uuid TEXT NOT NULL REFERENCES users(user_uuid),
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE sites (
    site_uuid TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE devices (
    device_uuid TEXT PRIMARY KEY NOT NULL,
    rustdesk_id TEXT,
    alias TEXT NOT NULL,
    hostname TEXT,
    os_family TEXT,
    os_version TEXT,
    architecture TEXT,
    rustdesk_version TEXT,
    site_uuid TEXT REFERENCES sites(site_uuid),
    owner TEXT,
    notes TEXT,
    last_checkin_at TEXT,
    last_lan_ip TEXT,
    last_wan_ip TEXT,
    archived INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_devices_rustdesk_id ON devices(rustdesk_id);
CREATE INDEX idx_devices_hostname ON devices(hostname);
CREATE INDEX idx_devices_archived ON devices(archived);

CREATE TABLE enrollment_tokens (
    enrollment_token_uuid TEXT PRIMARY KEY NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    label TEXT NOT NULL,
    site_uuid TEXT REFERENCES sites(site_uuid),
    expires_at TEXT,
    revoked_at TEXT,
    created_at TEXT NOT NULL,
    created_by_user_uuid TEXT REFERENCES users(user_uuid)
);

CREATE TABLE endpoint_checkins (
    endpoint_checkin_uuid TEXT PRIMARY KEY NOT NULL,
    device_uuid TEXT NOT NULL REFERENCES devices(device_uuid),
    enrollment_token_uuid TEXT NOT NULL REFERENCES enrollment_tokens(enrollment_token_uuid),
    rustdesk_id TEXT,
    hostname TEXT,
    os_family TEXT,
    os_version TEXT,
    architecture TEXT,
    rustdesk_version TEXT,
    lan_ip TEXT,
    wan_ip TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE server_configs (
    server_config_uuid TEXT PRIMARY KEY NOT NULL,
    id_server TEXT NOT NULL,
    relay_server TEXT NOT NULL,
    api_server TEXT NOT NULL DEFAULT '',
    public_key TEXT NOT NULL DEFAULT '',
    updated_at TEXT NOT NULL,
    updated_by_user_uuid TEXT REFERENCES users(user_uuid)
);

CREATE TABLE audit_events (
    audit_event_uuid TEXT PRIMARY KEY NOT NULL,
    actor_user_uuid TEXT,
    action TEXT NOT NULL,
    object_type TEXT NOT NULL,
    object_uuid TEXT,
    outcome TEXT NOT NULL,
    source TEXT NOT NULL,
    detail_json TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX idx_audit_events_created_at ON audit_events(created_at);