use sqlx::SqlitePool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::backup::BackupDocument;
use crate::time_format::format_timestamp;

use super::backup::BackupRestoreError;
use super::backup_address_book_tags::restore_address_book_tags;
use super::backup_address_books::restore_address_book_access_rules;

pub async fn restore_backup_document(
    pool: &SqlitePool,
    document: &BackupDocument,
) -> Result<(), BackupRestoreError> {
    crate::domain::backup::validate_backup_document(document)?;
    let pending: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM migration_legacy_credentials WHERE consumed_at IS NULL",
    )
    .fetch_one(pool)
    .await?;
    if pending != 0 {
        return Err(BackupRestoreError::UnconsumedMigrationVerifiers);
    }
    let now = format_timestamp(OffsetDateTime::now_utc());
    let mut tx = pool.begin().await?;
    for table in [
        "device_tags",
        "address_book_entry_tags",
        "address_book_tags",
        "address_book_access_rules",
        "address_book_entries",
        "address_books",
        "user_device_visibility_grants",
        "device_visibility_grants",
        "access_group_memberships",
        "access_groups",
        "endpoint_checkins",
        "devices",
        "enrollment_tokens",
        "tags",
        "sites",
        "server_configs",
        "sessions",
        "client_access_tokens",
        "migration_activation_tokens",
        "migration_legacy_credentials",
        "migration_credential_attachment_receipts",
        "migration_source_bindings",
        "migration_runs",
        "users",
    ] {
        sqlx::query(&format!("DELETE FROM {table}"))
            .execute(&mut *tx)
            .await?;
    }

    for site in &document.sites {
        sqlx::query(
            "INSERT INTO sites (site_uuid, name, created_at, updated_at) VALUES (?, ?, ?, ?)",
        )
        .bind(site.site_uuid.to_string())
        .bind(&site.name)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    }
    for tag in &document.tags {
        sqlx::query(
            "INSERT INTO tags (tag_uuid, name, created_at, updated_at) VALUES (?, ?, ?, ?)",
        )
        .bind(tag.tag_uuid.to_string())
        .bind(&tag.name)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    }
    for device in &document.devices {
        sqlx::query(
            "INSERT INTO devices (
                device_uuid, rustdesk_id, alias, hostname, os_family, os_version, architecture,
                rustdesk_version, site_uuid, owner, notes, last_checkin_at, archived,
                created_at, updated_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(device.device_uuid.to_string())
        .bind(device.rustdesk_id.as_deref())
        .bind(&device.alias)
        .bind(device.hostname.as_deref())
        .bind(device.os_family.as_deref())
        .bind(device.os_version.as_deref())
        .bind(device.architecture.as_deref())
        .bind(device.rustdesk_version.as_deref())
        .bind(device.site_uuid.map(|value| value.to_string()))
        .bind(device.owner.as_deref())
        .bind(device.notes.as_deref())
        .bind(device.last_checkin_at.as_deref())
        .bind(if device.archived { 1 } else { 0 })
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    }
    for link in &document.device_tags {
        sqlx::query("INSERT INTO device_tags (device_uuid, tag_uuid) VALUES (?, ?)")
            .bind(link.device_uuid.to_string())
            .bind(link.tag_uuid.to_string())
            .execute(&mut *tx)
            .await?;
    }
    for user in &document.users {
        sqlx::query(
            "INSERT INTO users (user_uuid, username, password_hash, role, activation_state, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(user.user_uuid.to_string())
        .bind(&user.username)
        .bind(&user.password_hash)
        .bind(&user.role)
        .bind(&user.activation_state)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    }
    for group in &document.access_groups {
        sqlx::query("INSERT INTO access_groups (access_group_uuid, name) VALUES (?, ?)")
            .bind(group.access_group_uuid.to_string())
            .bind(&group.name)
            .execute(&mut *tx)
            .await?;
    }
    for book in &document.address_books {
        sqlx::query(
            "INSERT INTO address_books (address_book_uuid, owner_user_uuid, name, book_kind)
             VALUES (?, ?, ?, ?)",
        )
        .bind(book.address_book_uuid.to_string())
        .bind(book.owner_user_uuid.to_string())
        .bind(&book.name)
        .bind(&book.book_kind)
        .execute(&mut *tx)
        .await?;
    }
    for membership in &document.access_group_memberships {
        sqlx::query(
            "INSERT INTO access_group_memberships (access_group_uuid, user_uuid) VALUES (?, ?)",
        )
        .bind(membership.access_group_uuid.to_string())
        .bind(membership.user_uuid.to_string())
        .execute(&mut *tx)
        .await?;
    }
    for grant in &document.device_visibility_grants {
        sqlx::query(
            "INSERT INTO device_visibility_grants (access_group_uuid, device_uuid) VALUES (?, ?)",
        )
        .bind(grant.access_group_uuid.to_string())
        .bind(grant.device_uuid.to_string())
        .execute(&mut *tx)
        .await?;
    }
    for grant in &document.user_device_visibility_grants {
        sqlx::query(
            "INSERT INTO user_device_visibility_grants (user_uuid, device_uuid) VALUES (?, ?)",
        )
        .bind(grant.user_uuid.to_string())
        .bind(grant.device_uuid.to_string())
        .execute(&mut *tx)
        .await?;
    }
    restore_address_book_access_rules(&mut tx, &document.address_book_access_rules).await?;
    for entry in &document.address_book_entries {
        sqlx::query(
            "INSERT INTO address_book_entries (
                address_book_entry_uuid, address_book_uuid, device_uuid, alias, notes, position
             ) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(entry.address_book_entry_uuid.to_string())
        .bind(entry.address_book_uuid.to_string())
        .bind(entry.device_uuid.to_string())
        .bind(&entry.alias)
        .bind(entry.notes.as_deref())
        .bind(i64::from(entry.position))
        .execute(&mut *tx)
        .await?;
    }
    restore_address_book_tags(
        &mut tx,
        &document.address_book_tags,
        &document.address_book_entry_tags,
    )
    .await?;
    if let Some(config) = &document.server_config {
        sqlx::query(
            "INSERT INTO server_configs (
                server_config_uuid, id_server, relay_server, api_server, public_key,
                updated_at, updated_by_user_uuid
             ) VALUES (?, ?, ?, ?, ?, ?, NULL)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&config.id_server)
        .bind(&config.relay_server)
        .bind(&config.api_server)
        .bind(&config.public_key)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    }
    for token in &document.enrollment_tokens {
        sqlx::query(
            "INSERT INTO enrollment_tokens (
                enrollment_token_uuid, token_hash, label, site_uuid, expires_at, revoked_at, created_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(token.enrollment_token_uuid.to_string())
        .bind(&token.token_hash)
        .bind(&token.label)
        .bind(token.site_uuid.map(|value| value.to_string()))
        .bind(token.expires_at.as_deref())
        .bind(token.revoked_at.as_deref())
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}
