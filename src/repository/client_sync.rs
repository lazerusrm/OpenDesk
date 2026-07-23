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
}

pub async fn list_client_visible_peers(
    pool: &SqlitePool,
    user_uuid: Uuid,
) -> Result<Vec<ClientPeerRow>, sqlx::Error> {
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
        ),
    >(
        "SELECT device.rustdesk_id, device.alias, device.hostname, device.os_family,
                device.os_version, device.owner, device.notes
         FROM devices device
         WHERE device.archived = 0 AND device.rustdesk_id IS NOT NULL
           AND (
               EXISTS (
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
               )
           )
         ORDER BY device.rustdesk_id ASC",
    )
    .bind(user_uuid.to_string())
    .bind(user_uuid.to_string())
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(
            |(rustdesk_id, alias, hostname, os_family, os_version, owner, notes)| ClientPeerRow {
                rustdesk_id,
                alias,
                hostname,
                os_family,
                os_version,
                owner,
                notes,
            },
        )
        .collect())
}

pub async fn list_client_access_groups(
    pool: &SqlitePool,
    user_uuid: Uuid,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT access_group.name
         FROM access_groups access_group
         JOIN access_group_memberships membership
           ON membership.access_group_uuid = access_group.access_group_uuid
         WHERE membership.user_uuid = ?
         ORDER BY access_group.name ASC",
    )
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
                   SELECT 1 FROM user_device_visibility_grants direct_grant
                   WHERE direct_grant.user_uuid = ?
                     AND direct_grant.device_uuid = device.device_uuid
               ) OR EXISTS (
                   SELECT 1 FROM access_group_memberships membership
                   JOIN device_visibility_grants group_grant
                     ON group_grant.access_group_uuid = membership.access_group_uuid
                   WHERE membership.user_uuid = ?
                     AND group_grant.device_uuid = device.device_uuid
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
               accessible.user_uuid = ? OR EXISTS (
                   SELECT 1
                   FROM access_group_memberships own_membership
                   JOIN access_group_memberships peer_membership
                     ON peer_membership.access_group_uuid = own_membership.access_group_uuid
                   WHERE own_membership.user_uuid = ?
                     AND peer_membership.user_uuid = accessible.user_uuid
               )
           )
         ORDER BY accessible.username ASC",
    )
    .bind(user_uuid.to_string())
    .bind(user_uuid.to_string())
    .fetch_all(pool)
    .await
}
