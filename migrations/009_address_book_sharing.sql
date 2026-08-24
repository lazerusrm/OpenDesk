ALTER TABLE address_books ADD COLUMN book_kind TEXT NOT NULL DEFAULT 'personal'
    CHECK (book_kind IN ('personal', 'shared'));

CREATE TABLE address_book_access_rules (
    address_book_uuid TEXT NOT NULL REFERENCES address_books(address_book_uuid) ON DELETE CASCADE,
    principal_type TEXT NOT NULL CHECK (principal_type IN ('user', 'group')),
    principal_uuid TEXT NOT NULL,
    permission TEXT NOT NULL CHECK (permission IN ('read', 'write', 'admin')),
    PRIMARY KEY (address_book_uuid, principal_type, principal_uuid)
);

CREATE TABLE address_book_tags (
    address_book_uuid TEXT NOT NULL REFERENCES address_books(address_book_uuid) ON DELETE CASCADE,
    name TEXT NOT NULL,
    color INTEGER NOT NULL,
    PRIMARY KEY (address_book_uuid, name)
);

CREATE TABLE address_book_entry_tags (
    address_book_entry_uuid TEXT NOT NULL REFERENCES address_book_entries(address_book_entry_uuid) ON DELETE CASCADE,
    tag_name TEXT NOT NULL,
    address_book_uuid TEXT NOT NULL,
    PRIMARY KEY (address_book_entry_uuid, tag_name),
    FOREIGN KEY (address_book_uuid, tag_name)
        REFERENCES address_book_tags(address_book_uuid, name) ON DELETE CASCADE
);

CREATE INDEX idx_address_books_owner_kind
    ON address_books(owner_user_uuid, book_kind);
CREATE INDEX idx_address_book_access_rules_principal
    ON address_book_access_rules(principal_type, principal_uuid);
