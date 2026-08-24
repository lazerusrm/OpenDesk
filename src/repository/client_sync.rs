use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(Debug)]
pub struct ClientPeerRow {
    pub rustdesk_id: String,
    pub alias: String,
    pub hostname: Option<String>,
    pub os_family: Option<String>,
    pub os_version: Option<String>,
    pub owner: Option<String>,
    pub notes: Option<String>,
    pub user_guid: String,
    pub user_name: String,
    pub device_group_name: String,
}

pub async fn list_client_visible_peers(
    pool: &SqlitePool,
    user_uuid: Uuid,
) -> Result<Vec<ClientPeerRow>, sqlx::Error> {
    let viewer = user_uuid.to_string();
    let rows = sqlx::query_as::<
        _,
        (
            String,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            String,
            String,
            String,
        ),
    >(
        "SELECT device.rustdesk_id, device.alias, device.hostname, device.os_family,
                device.os_version, device.owner, device.notes,
                COALESCE(
                    (SELECT owner_user.user_uuid FROM users owner_user
                     WHERE owner_user.username = device.owner
                       AND owner_user.activation_state = 'active'
                     LIMIT 1),
                    (SELECT grant_user.user_uuid
                     FROM user_device_visibility_grants direct_grant
                     JOIN users grant_user ON grant_user.user_uuid = direct_grant.user_uuid
                     WHERE direct_grant.device_uuid = device.device_uuid
                       AND grant_user.activation_state = 'active'
                     ORDER BY grant_user.username
                     LIMIT 1),
                    ''
                ),
                COALESCE(
                    (SELECT owner_user.username FROM users owner_user
                     WHERE owner_user.username = device.owner
                       AND owner_user.activation_state = 'active'
                     LIMIT 1),
                    (SELECT grant_user.username
                     FROM user_device_visibility_grants direct_grant
                     JOIN users grant_user ON grant_user.user_uuid = direct_grant.user_uuid
                     WHERE direct_grant.device_uuid = device.device_uuid
                       AND grant_user.activation_state = 'active'
                     ORDER BY grant_user.username
                     LIMIT 1),
                    ''
                ),
                COALESCE(
                    (SELECT access_group.name
                     FROM device_visibility_grants group_grant
                     JOIN access_groups access_group
                       ON access_group.access_group_uuid = group_grant.access_group_uuid
                     WHERE group_grant.device_uuid = device.device_uuid
                     ORDER BY CASE WHEN EXISTS (
                         SELECT 1 FROM access_group_memberships viewer_membership
                         WHERE viewer_membership.access_group_uuid = group_grant.access_group_uuid
                           AND viewer_membership.user_uuid = ?
                     ) THEN 0 ELSE 1 END,
                     access_group.name
                     LIMIT 1),
                    ''
                )
         FROM devices device
         WHERE device.archived = 0 AND device.rustdesk_id IS NOT NULL
           AND (
               EXISTS (
                   SELECT 1 FROM users visibility_admin
                   WHERE visibility_admin.user_uuid = ?
                     AND visibility_admin.role = 'admin'
               ) OR EXISTS (
                   SELECT 1 FROM user_device_visibility_grants direct_grant
                   WHERE direct_grant.user_uuid = ?
                     AND direct_grant.device_uuid = device.device_uuid
               ) OR EXISTS (
                   SELECT 1
                   FROM access_group_memberships membership
                   JOIN device_visibility_grants group_grant
                     ON group_grant.access_group_uuid = membership.access_group_uuid
                   WHERE membership.user_uuid = ?
                     AND group_grant.device_uuid = device.device_uuid
               ) OR EXISTS (
                   SELECT 1
                   FROM access_group_memberships incoming_membership
                   JOIN access_group_access_grants group_access
                     ON group_access.incoming_access_group_uuid = incoming_membership.access_group_uuid
                   JOIN device_visibility_grants outgoing_grant
                     ON outgoing_grant.access_group_uuid = group_access.outgoing_access_group_uuid
                   WHERE incoming_membership.user_uuid = ?
                     AND outgoing_grant.device_uuid = device.device_uuid
               )
           )
         ORDER BY device.rustdesk_id ASC",
    )
    .bind(&viewer)
    .bind(&viewer)
    .bind(&viewer)
    .bind(&viewer)
    .bind(&viewer)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(
            |(
                rustdesk_id,
                alias,
                hostname,
                os_family,
                os_version,
                owner,
                notes,
                user_guid,
                user_name,
                device_group_name,
            )| ClientPeerRow {
                rustdesk_id,
                alias,
                hostname,
                os_family,
                os_version,
                owner,
                notes,
                user_guid,
                user_name,
                device_group_name,
            },
        )
        .collect())
}

pub async fn list_client_access_groups(
    pool: &SqlitePool,
    user_uuid: Uuid,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT name FROM (
             SELECT access_group.name AS name
             FROM access_groups access_group
             JOIN access_group_memberships membership
               ON membership.access_group_uuid = access_group.access_group_uuid
             WHERE membership.user_uuid = ?
             UNION
             SELECT outgoing.name AS name
             FROM access_group_memberships membership
             JOIN access_group_access_grants group_access
               ON group_access.incoming_access_group_uuid = membership.access_group_uuid
             JOIN access_groups outgoing
               ON outgoing.access_group_uuid = group_access.outgoing_access_group_uuid
             WHERE membership.user_uuid = ?
         )
         ORDER BY name ASC",
    )
    .bind(user_uuid.to_string())
    .bind(user_uuid.to_string())
    .fetch_all(pool)
    .await
}

pub struct ClientAddressBookPeerRow {
    pub rustdesk_id: String,
    pub alias: String,
    pub hostname: Option<String>,
    pub os_family: Option<String>,
    pub owner: Option<String>,
    pub notes: Option<String>,
    pub tags: Vec<String>,
}

pub async fn list_client_address_book_peers(
    pool: &SqlitePool,
    user_uuid: Uuid,
    address_book_uuid: Uuid,
) -> Result<Vec<ClientAddressBookPeerRow>, sqlx::Error> {
    let rows = sqlx::query_as::<
        _,
        (
            String,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            String,
        ),
    >(
        "SELECT device.rustdesk_id, entry.alias, device.hostname, device.os_family,
                device.owner, entry.notes,
                COALESCE((SELECT json_group_array(tag_name) FROM (
                    SELECT tag_name FROM address_book_entry_tags
                    WHERE address_book_entry_uuid = entry.address_book_entry_uuid
                    ORDER BY tag_name
                )), '[]')
         FROM address_book_entries entry
         JOIN address_books book ON book.address_book_uuid = entry.address_book_uuid
         JOIN devices device ON device.device_uuid = entry.device_uuid
         WHERE book.address_book_uuid = ?
           AND (
               book.owner_user_uuid = ? OR EXISTS (
                   SELECT 1 FROM address_book_access_rules rule
                   WHERE rule.address_book_uuid = book.address_book_uuid
                     AND (
                         (rule.principal_type = 'user' AND rule.principal_uuid = ?)
                         OR (rule.principal_type = 'group' AND EXISTS (
                             SELECT 1 FROM access_group_memberships book_membership
                             WHERE book_membership.user_uuid = ?
                               AND book_membership.access_group_uuid = rule.principal_uuid
                         ))
                     )
               )
           )
           AND device.archived = 0 AND device.rustdesk_id IS NOT NULL
           AND (
               EXISTS (
                   SELECT 1 FROM users visibility_admin
                   WHERE visibility_admin.user_uuid = ?
                     AND visibility_admin.role = 'admin'
               ) OR EXISTS (
                   SELECT 1 FROM user_device_visibility_grants direct_grant
                   WHERE direct_grant.user_uuid = ?
                     AND direct_grant.device_uuid = device.device_uuid
               ) OR EXISTS (
                   SELECT 1 FROM access_group_memberships membership
                   JOIN device_visibility_grants group_grant
                     ON group_grant.access_group_uuid = membership.access_group_uuid
                   WHERE membership.user_uuid = ?
                     AND group_grant.device_uuid = device.device_uuid
               ) OR EXISTS (
                   SELECT 1 FROM access_group_memberships incoming_membership
                   JOIN access_group_access_grants group_access
                     ON group_access.incoming_access_group_uuid = incoming_membership.access_group_uuid
                   JOIN device_visibility_grants outgoing_grant
                     ON outgoing_grant.access_group_uuid = group_access.outgoing_access_group_uuid
                   WHERE incoming_membership.user_uuid = ?
                     AND outgoing_grant.device_uuid = device.device_uuid
               )
           )
         ORDER BY entry.position ASC, entry.address_book_entry_uuid ASC",
    )
    .bind(address_book_uuid.to_string())
    .bind(user_uuid.to_string())
    .bind(user_uuid.to_string())
    .bind(user_uuid.to_string())
    .bind(user_uuid.to_string())
    .bind(user_uuid.to_string())
    .bind(user_uuid.to_string())
    .bind(user_uuid.to_string())
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(
            |(rustdesk_id, alias, hostname, os_family, owner, notes, tags)| {
                ClientAddressBookPeerRow {
                    rustdesk_id,
                    alias,
                    hostname,
                    os_family,
                    owner,
                    notes,
                    tags: serde_json::from_str(&tags).expect("stored tag JSON"),
                }
            },
        )
        .collect())
}

pub async fn list_client_accessible_users(
    pool: &SqlitePool,
    user_uuid: Uuid,
) -> Result<Vec<(String, String, String)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT DISTINCT accessible.user_uuid, accessible.username, accessible.role
         FROM users accessible
         WHERE accessible.activation_state = 'active'
           AND (
               accessible.user_uuid = ?
               OR EXISTS (
                   SELECT 1 FROM users visibility_admin
                   WHERE visibility_admin.user_uuid = ?
                     AND visibility_admin.role = 'admin'
               )
               OR EXISTS (
                   SELECT 1
                   FROM access_group_memberships own_membership
                   JOIN access_group_memberships peer_membership
                     ON peer_membership.access_group_uuid = own_membership.access_group_uuid
                   WHERE own_membership.user_uuid = ?
                     AND peer_membership.user_uuid = accessible.user_uuid
               )
               OR EXISTS (
                   SELECT 1
                   FROM access_group_memberships own_membership
                   JOIN access_group_access_grants group_access
                     ON group_access.incoming_access_group_uuid = own_membership.access_group_uuid
                   JOIN access_group_memberships peer_membership
                     ON peer_membership.access_group_uuid = group_access.outgoing_access_group_uuid
                   WHERE own_membership.user_uuid = ?
                     AND peer_membership.user_uuid = accessible.user_uuid
               )
           )
         ORDER BY accessible.username ASC",
    )
    .bind(user_uuid.to_string())
    .bind(user_uuid.to_string())
    .bind(user_uuid.to_string())
    .bind(user_uuid.to_string())
    .fetch_all(pool)
    .await
}
