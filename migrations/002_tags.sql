CREATE TABLE tags (
    tag_uuid TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE device_tags (
    device_uuid TEXT NOT NULL REFERENCES devices(device_uuid) ON DELETE CASCADE,
    tag_uuid TEXT NOT NULL REFERENCES tags(tag_uuid) ON DELETE CASCADE,
    PRIMARY KEY (device_uuid, tag_uuid)
);

CREATE INDEX idx_device_tags_tag_uuid ON device_tags(tag_uuid);