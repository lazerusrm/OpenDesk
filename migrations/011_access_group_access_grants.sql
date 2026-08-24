CREATE TABLE access_group_access_grants (
    incoming_access_group_uuid TEXT NOT NULL
        REFERENCES access_groups(access_group_uuid) ON DELETE CASCADE,
    outgoing_access_group_uuid TEXT NOT NULL
        REFERENCES access_groups(access_group_uuid) ON DELETE CASCADE,
    PRIMARY KEY (incoming_access_group_uuid, outgoing_access_group_uuid),
    CHECK (incoming_access_group_uuid != outgoing_access_group_uuid)
);

CREATE INDEX idx_access_group_access_grants_outgoing
    ON access_group_access_grants(outgoing_access_group_uuid);
