use std::collections::HashSet;

use sqlx::sqlite::SqliteConnection;
use thiserror::Error;
use uuid::Uuid;

use super::super::{device::Device, site::Site, user::User};
use super::{MigrationSnapshot, RustDeskProImportDocument, ScopeSiteMapping};

const MAX_GROUP_ID_LENGTH: usize = 128;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ScopeMappingParseError {
    #[error("mapping must be exactly group-id:site-UUID")]
    InvalidFormat,
    #[error("mapping group id must not be empty")]
    EmptyGroupId,
    #[error("mapping group id contains unsafe characters")]
    UnsafeGroupId,
    #[error("mapping site UUID must be canonical")]
    InvalidSiteUuid,
}

/// Parse the only operator mapping syntax accepted by the dry-run binary.
/// Group IDs are treated as opaque external identifiers, but delimiters,
/// whitespace, and control characters are rejected so input cannot smuggle
/// another mapping or a path-like value into the boundary.
pub fn parse_scope_site_mapping(value: &str) -> Result<ScopeSiteMapping, ScopeMappingParseError> {
    let Some((group_id, site_uuid)) = value.split_once(':') else {
        return Err(ScopeMappingParseError::InvalidFormat);
    };
    if site_uuid.contains(':') {
        return Err(ScopeMappingParseError::InvalidFormat);
    }
    if group_id.is_empty() {
        return Err(ScopeMappingParseError::EmptyGroupId);
    }
    if group_id.len() > MAX_GROUP_ID_LENGTH
        || !group_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
    {
        return Err(ScopeMappingParseError::UnsafeGroupId);
    }
    let Ok(site_uuid_value) = Uuid::parse_str(site_uuid) else {
        return Err(ScopeMappingParseError::InvalidSiteUuid);
    };
    if site_uuid_value.to_string() != site_uuid {
        return Err(ScopeMappingParseError::InvalidSiteUuid);
    }
    Ok(ScopeSiteMapping {
        group_id: group_id.to_string(),
        site_uuid: site_uuid_value,
    })
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ScopeMappingValidationError {
    #[error("mapping contains duplicate group id")]
    DuplicateGroupId,
    #[error("mapping references an unknown import group")]
    UnknownGroupId,
    #[error("mapping references an unknown current site")]
    UnknownSiteUuid,
}

/// Validate mappings against both inputs before they are attached to a
/// snapshot. Names are deliberately not consulted: only exact IDs and UUIDs
/// supplied by the operator are accepted.
pub fn validate_scope_site_mappings(
    document: &RustDeskProImportDocument,
    snapshot: &MigrationSnapshot,
    mappings: &[ScopeSiteMapping],
) -> Result<(), ScopeMappingValidationError> {
    let group_ids: HashSet<&str> = document
        .groups
        .iter()
        .map(|group| group.group_id.as_str())
        .collect();
    let site_ids: HashSet<Uuid> = snapshot.sites.iter().map(|site| site.site_uuid).collect();
    let mut seen_group_ids = HashSet::new();
    for mapping in mappings {
        if !seen_group_ids.insert(mapping.group_id.as_str()) {
            return Err(ScopeMappingValidationError::DuplicateGroupId);
        }
        if !group_ids.contains(mapping.group_id.as_str()) {
            return Err(ScopeMappingValidationError::UnknownGroupId);
        }
        if !site_ids.contains(&mapping.site_uuid) {
            return Err(ScopeMappingValidationError::UnknownSiteUuid);
        }
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum SnapshotLoadError {
    #[error("current database snapshot query failed")]
    Database(#[from] sqlx::Error),
    #[error("current database contains an invalid {table} UUID")]
    InvalidUuid { table: &'static str },
}

type DeviceRow = (
    String,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    i64,
);

fn parse_uuid(value: String, table: &'static str) -> Result<Uuid, SnapshotLoadError> {
    Uuid::parse_str(&value).map_err(|_| SnapshotLoadError::InvalidUuid { table })
}

/// Read the current OpenDesk state using one caller-owned, read-only
/// connection. This function selects no password, token, session, audit, or
/// endpoint-check-in fields.
pub async fn load_migration_snapshot(
    connection: &mut SqliteConnection,
) -> Result<MigrationSnapshot, SnapshotLoadError> {
    let user_rows = sqlx::query_as::<_, (String, String, String)>(
        "SELECT user_uuid, username, role FROM users ORDER BY user_uuid ASC",
    )
    .fetch_all(&mut *connection)
    .await?;
    let users = user_rows
        .into_iter()
        .map(|(user_uuid, username, role)| {
            Ok(User {
                user_uuid: parse_uuid(user_uuid, "users")?,
                username,
                role,
            })
        })
        .collect::<Result<Vec<_>, SnapshotLoadError>>()?;

    let site_rows = sqlx::query_as::<_, (String, String)>(
        "SELECT site_uuid, name FROM sites ORDER BY site_uuid ASC",
    )
    .fetch_all(&mut *connection)
    .await?;
    let sites = site_rows
        .into_iter()
        .map(|(site_uuid, name)| {
            Ok(Site {
                site_uuid: parse_uuid(site_uuid, "sites")?,
                name,
            })
        })
        .collect::<Result<Vec<_>, SnapshotLoadError>>()?;

    let device_rows = sqlx::query_as::<_, DeviceRow>(
        "SELECT device_uuid, rustdesk_id, alias, hostname, os_family, os_version,
                architecture, rustdesk_version, site_uuid, owner, notes, last_checkin_at, archived
         FROM devices ORDER BY device_uuid ASC",
    )
    .fetch_all(&mut *connection)
    .await?;
    let devices = device_rows
        .into_iter()
        .map(
            |(
                device_uuid,
                rustdesk_id,
                alias,
                hostname,
                os_family,
                os_version,
                architecture,
                rustdesk_version,
                site_uuid,
                owner,
                notes,
                last_checkin_at,
                archived,
            )| {
                Ok(Device {
                    device_uuid: parse_uuid(device_uuid, "devices")?,
                    rustdesk_id,
                    alias,
                    hostname,
                    os_family,
                    os_version,
                    architecture,
                    rustdesk_version,
                    site_uuid: site_uuid
                        .map(|value| parse_uuid(value, "devices"))
                        .transpose()?,
                    owner,
                    notes,
                    archived: archived != 0,
                    last_checkin_at,
                })
            },
        )
        .collect::<Result<Vec<_>, SnapshotLoadError>>()?;

    Ok(MigrationSnapshot {
        users,
        sites,
        devices,
        scope_site_mappings: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mapping_parser_requires_canonical_uuid_and_safe_id() {
        let uuid = Uuid::new_v4().to_string();
        assert!(parse_scope_site_mapping(&format!("group-1:{uuid}")).is_ok());
        assert_eq!(
            parse_scope_site_mapping(&format!("group:1:{uuid}")),
            Err(ScopeMappingParseError::InvalidFormat)
        );
        assert_eq!(
            parse_scope_site_mapping(&format!("group one:{uuid}")),
            Err(ScopeMappingParseError::UnsafeGroupId)
        );
        assert_eq!(
            parse_scope_site_mapping("group-1:not-a-uuid"),
            Err(ScopeMappingParseError::InvalidSiteUuid)
        );
    }

    #[test]
    fn mappings_reject_duplicates_and_unknown_ids() {
        let site_uuid = Uuid::new_v4();
        let document = RustDeskProImportDocument {
            schema_version: 1,
            users: vec![],
            groups: vec![super::super::RustDeskProGroup {
                group_id: "group-1".to_string(),
                name: "Name is not consulted".to_string(),
            }],
            devices: vec![],
            address_books: vec![],
            address_book_entries: vec![],
            cross_group_edges: vec![],
        };
        let snapshot = MigrationSnapshot {
            sites: vec![Site {
                site_uuid,
                name: "Different name".to_string(),
            }],
            ..Default::default()
        };
        let mapping = ScopeSiteMapping {
            group_id: "group-1".to_string(),
            site_uuid,
        };
        assert!(validate_scope_site_mappings(&document, &snapshot, &[mapping.clone()]).is_ok());
        assert_eq!(
            validate_scope_site_mappings(&document, &snapshot, &[mapping.clone(), mapping]),
            Err(ScopeMappingValidationError::DuplicateGroupId)
        );
        assert_eq!(
            validate_scope_site_mappings(
                &document,
                &snapshot,
                &[ScopeSiteMapping {
                    group_id: "missing".to_string(),
                    site_uuid,
                }]
            ),
            Err(ScopeMappingValidationError::UnknownGroupId)
        );
    }
}
