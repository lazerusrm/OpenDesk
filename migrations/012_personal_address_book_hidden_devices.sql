CREATE TABLE personal_address_book_hidden_devices (
    owner_user_uuid TEXT NOT NULL REFERENCES users(user_uuid) ON DELETE CASCADE,
    device_uuid TEXT NOT NULL REFERENCES devices(device_uuid) ON DELETE CASCADE,
    PRIMARY KEY (owner_user_uuid, device_uuid)
);
