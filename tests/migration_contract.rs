use opendesk::domain::migration_contract::*;
use uuid::Uuid;

fn sample() -> SanitizedMigrationExport {
    let digest = "a".repeat(64);
    SanitizedMigrationExport {
        schema_version: SANITIZED_MIGRATION_SCHEMA_VERSION,
        provenance: SourceProvenance {
            source_system: "rustdesk_pro".into(),
            source_instance: "source-1".into(),
            source_export_id: "export-1".into(),
            source_schema_version: "7".into(),
            exported_at: "2026-07-22T12:00:00Z".into(),
            snapshot_sha256: digest.clone(),
        },
        run: MigrationRunState {
            run_id: Uuid::from_u128(1),
            status: MigrationRunStatus::DryRunReady,
            started_at: "2026-07-22T12:00:00Z".into(),
            completed_at: Some("2026-07-22T12:01:00Z".into()),
            source_snapshot_sha256: digest,
        },
        users: vec![SourceUser {
            source_user_id: "user-1".into(),
            username: "operator".into(),
            role: None,
            credential_reset_required: true,
        }],
        groups: vec![SourceGroup {
            source_group_id: "group-1".into(),
            name: "Ops".into(),
            allow_device_access_within_group: true,
        }],
        user_group_memberships: vec![SourceUserGroupMembership {
            source_user_id: "user-1".into(),
            source_group_id: "group-1".into(),
        }],
        cross_group_access: vec![],
        devices: vec![SourceDevice {
            rustdesk_id: "123".into(),
            alias: "Desktop".into(),
            hostname: None,
            owner_source_user_id: Some("user-1".into()),
            source_group_ids: vec![],
        }],
        address_books: vec![SourceAddressBook {
            source_address_book_id: "book-1".into(),
            name: "Ops".into(),
            owner_source_user_id: Some("user-1".into()),
            book_kind: AddressBookKind::Shared,
            rules: vec![SourceAddressBookRule {
                principal_type: AddressBookPrincipalType::Group,
                principal_id: "group-1".into(),
                permission: AddressBookPermission::Read,
            }],
        }],
        address_book_entries: vec![SourceAddressBookEntry {
            source_address_book_id: "book-1".into(),
            rustdesk_id: "123".into(),
            alias: "Desktop".into(),
            notes: None,
            credential_reset_required: true,
        }],
        settings: vec![SourceSetting {
            key: "theme".into(),
            disposition: SettingsDisposition::Exclude,
        }],
        unsupported_semantics: vec![],
    }
}

#[test]
fn export_round_trip_preserves_relationships_and_dispositions() {
    let value = serde_json::to_string(&sample()).expect("serialize");
    assert_eq!(parse_sanitized_export(&value).expect("parse"), sample());
}

#[test]
fn parser_rejects_hashes_and_unknown_fields() {
    let mut value = serde_json::to_value(sample()).expect("value");
    value["users"][0]["password_hash"] = serde_json::json!("never");
    let json = serde_json::to_string(&value).expect("serialize");
    assert_eq!(
        parse_sanitized_export(&json),
        Err(MigrationContractError::InvalidSourceValue { field: "sensitive" })
    );
    let json = serde_json::to_string(&serde_json::json!({"unexpected": true})).expect("serialize");
    assert_eq!(
        parse_sanitized_export(&json),
        Err(MigrationContractError::InvalidSourceValue { field: "schema" })
    );
}

#[test]
fn validation_requires_reset_and_known_relationships() {
    let mut document = sample();
    document.users[0].credential_reset_required = false;
    assert_eq!(
        validate_export(&document),
        Err(MigrationContractError::CredentialResetRequired)
    );
    let mut document = sample();
    document.user_group_memberships[0].source_group_id = "missing".into();
    let mut document = sample();
    document.unsupported_semantics = vec![
        UnsupportedSourceSemantics {
            category: UnsupportedSemanticsCategory::CustomClient,
            count: 5,
            disposition: UnsupportedSemanticsDisposition::Retired,
        },
        UnsupportedSourceSemantics {
            category: UnsupportedSemanticsCategory::CustomClient,
            count: 5,
            disposition: UnsupportedSemanticsDisposition::Retired,
        },
    ];
    assert_eq!(
        validate_export(&document),
        Err(MigrationContractError::InvalidSourceValue {
            field: "unsupported_semantics"
        })
    );
}
