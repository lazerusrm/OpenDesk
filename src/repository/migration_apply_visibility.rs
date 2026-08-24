use std::collections::{HashMap, HashSet};

use sqlx::SqliteConnection;

use crate::domain::{migration_contract::SanitizedMigrationExport, role::Role};

use super::{source_uuid, MigrationApplyError};

pub(super) async fn apply_device_visibility(
    connection: &mut SqliteConnection,
    export: &SanitizedMigrationExport,
    device_uuid: uuid::Uuid,
    owner_source_user_id: Option<&str>,
) -> Result<(), MigrationApplyError> {
    if let Some(owner_id) = owner_source_user_id {
        insert_direct_grant(connection, owner_id, device_uuid).await?;
        let user_groups: HashMap<&str, &str> = export
            .user_group_memberships
            .iter()
            .map(|membership| {
                (
                    membership.source_user_id.as_str(),
                    membership.source_group_id.as_str(),
                )
            })
            .collect();
        let owner_group = user_groups
            .get(owner_id)
            .copied()
            .ok_or(MigrationApplyError::UnsupportedSemantics)?;
        let allow_within_group = export.groups.iter().any(|group| {
            group.source_group_id == owner_group && group.allow_device_access_within_group
        });
        let mut visible_groups: HashSet<&str> = HashSet::new();
        if allow_within_group {
            visible_groups.insert(owner_group);
        }
        visible_groups.extend(
            export
                .cross_group_access
                .iter()
                .filter(|access| access.target_group_id == owner_group)
                .map(|access| access.source_group_id.as_str()),
        );
        for group_id in visible_groups {
            sqlx::query(
                "INSERT OR IGNORE INTO device_visibility_grants
                 (access_group_uuid, device_uuid) VALUES (?, ?)",
            )
            .bind(source_uuid("group", group_id).to_string())
            .bind(device_uuid.to_string())
            .execute(&mut *connection)
            .await?;
        }
    } else {
        for admin in export
            .users
            .iter()
            .filter(|user| user.role.as_deref() == Some(Role::ADMIN))
        {
            insert_direct_grant(connection, &admin.source_user_id, device_uuid).await?;
        }
    }
    Ok(())
}

async fn insert_direct_grant(
    connection: &mut SqliteConnection,
    source_user_id: &str,
    device_uuid: uuid::Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO user_device_visibility_grants (user_uuid, device_uuid) VALUES (?, ?)")
        .bind(source_uuid("user", source_user_id).to_string())
        .bind(device_uuid.to_string())
        .execute(connection)
        .await?;
    Ok(())
}
