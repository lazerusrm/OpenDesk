CREATE TABLE access_groups (
    access_group_uuid TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    CHECK (length(trim(name)) > 0),
    CHECK (name = trim(name)),
    CHECK (length(name) <= 128)
);

CREATE TABLE access_group_memberships (
    access_group_uuid TEXT NOT NULL REFERENCES access_groups(access_group_uuid) ON DELETE CASCADE,
    user_uuid TEXT NOT NULL REFERENCES users(user_uuid) ON DELETE CASCADE,
    PRIMARY KEY (access_group_uuid, user_uuid)
);

CREATE TABLE device_visibility_grants (
    access_group_uuid TEXT NOT NULL REFERENCES access_groups(access_group_uuid) ON DELETE CASCADE,
    device_uuid TEXT NOT NULL REFERENCES devices(device_uuid) ON DELETE CASCADE,
    PRIMARY KEY (access_group_uuid, device_uuid)
);

CREATE TABLE user_device_visibility_grants (
    user_uuid TEXT NOT NULL REFERENCES users(user_uuid) ON DELETE CASCADE,
    device_uuid TEXT NOT NULL REFERENCES devices(device_uuid) ON DELETE CASCADE,
    PRIMARY KEY (user_uuid, device_uuid)
);

CREATE TABLE address_books (
    address_book_uuid TEXT PRIMARY KEY NOT NULL,
    owner_user_uuid TEXT NOT NULL REFERENCES users(user_uuid) ON DELETE CASCADE,
    name TEXT NOT NULL,
    UNIQUE (owner_user_uuid, name),
    CHECK (length(trim(name)) > 0),
    CHECK (name = trim(name)),
    CHECK (length(name) <= 128)
);

CREATE TABLE address_book_entries (
    address_book_entry_uuid TEXT PRIMARY KEY NOT NULL,
    address_book_uuid TEXT NOT NULL REFERENCES address_books(address_book_uuid) ON DELETE CASCADE,
    device_uuid TEXT NOT NULL REFERENCES devices(device_uuid) ON DELETE CASCADE,
    alias TEXT NOT NULL,
    notes TEXT,
    position INTEGER NOT NULL,
    UNIQUE (address_book_uuid, device_uuid),
    CHECK (length(trim(alias)) > 0),
    CHECK (alias = trim(alias)),
    CHECK (length(alias) <= 128),
    CHECK (position >= 0)
);

CREATE INDEX idx_access_group_memberships_user_uuid
    ON access_group_memberships(user_uuid);
CREATE INDEX idx_device_visibility_grants_device_uuid
    ON device_visibility_grants(device_uuid);
CREATE INDEX idx_user_device_visibility_grants_device_uuid
    ON user_device_visibility_grants(device_uuid);
CREATE INDEX idx_address_books_owner_user_uuid
    ON address_books(owner_user_uuid);
CREATE INDEX idx_address_book_entries_device_uuid
    ON address_book_entries(device_uuid);
CREATE INDEX idx_address_book_entries_position
    ON address_book_entries(address_book_uuid, position);
