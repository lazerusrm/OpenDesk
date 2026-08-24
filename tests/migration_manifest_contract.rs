use opendesk::domain::migration::{
    dry_run_import, expected_import_counts, parse_approved_migration_manifest_json,
    parse_rustdesk_pro_import_json, planned_import_target, sha256_digest,
    validate_manifest_preconditions, ManifestParseError, ManifestPreconditionError,
    ManifestSemanticError, ManifestSourceCoverageError, MigrationSnapshot,
};
use opendesk::domain::user::User;
use serde_json::json;

fn manifest() -> serde_json::Value {
    json!({
        "schema_version": 1,
        "source_snapshot_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "expected_counts": {"users": 1, "groups": 0, "devices": 0, "address_books": 0, "address_book_entries": 0, "cross_group_edges": 0},
        "dry_run_report_sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "approver": "operator@example.invalid",
        "approved_at": "2026-01-01T00:00:00Z",
        "expires_at": "2026-02-01T00:00:00Z",
        "dispositions": [{
            "source_kind": "user", "source_id": "u1",
            "target_uuid": "00000000-0000-0000-0000-000000000001",
            "action": "merge", "identity_collision": "match_existing",
            "role_path": "ignore", "credential_path": "exclude",
            "visibility_intent": "unmapped"
        }]
    })
}

fn valid_would_create_book_manifest() -> serde_json::Value {
    let mut value = manifest();
    value["expected_counts"]["users"] = json!(0);
    value["expected_counts"]["address_books"] = json!(1);
    value["dispositions"][0]["source_kind"] = json!("address_book");
    value["dispositions"][0]["source_id"] = json!("book-1");
    value["dispositions"][0]["target_uuid"] = json!("00000000-0000-0000-0000-000000000002");
    value["dispositions"][0]["action"] = json!("import");
    value["dispositions"][0]["identity_collision"] = json!("create_new");
    value["dispositions"][0]["visibility_intent"] = json!("visible");
    value["dispositions"][0]["role_path"] = serde_json::Value::Null;
    value["dispositions"][0]["credential_path"] = serde_json::Value::Null;
    value
}

#[test]
fn would_create_address_book_accepts_planned_non_snapshot_target() {
    let value = valid_would_create_book_manifest();
    let document = serde_json::json!({"schema_version":1,"users":[],"groups":[],"devices":[],"address_books":[{"address_book_id":"book-1","name":"Book","group_id":null}],"address_book_entries":[]});
    let document = parse_rustdesk_pro_import_json(&document.to_string()).unwrap();
    let mut parsed = parse_approved_migration_manifest_json(&value.to_string()).unwrap();
    let mut disposition = parsed.dispositions.remove(0);
    disposition.target_uuid = Some(planned_import_target(&disposition));
    parsed.dispositions.push(disposition);
    let snapshot = MigrationSnapshot::default();
    let report = dry_run_import(&document, &snapshot);
    assert!(parsed
        .validate_against_snapshot_and_report(&snapshot, &report)
        .is_ok());
}

#[test]
fn manifest_rejects_nested_unknown_and_sensitive_fields() {
    let mut unknown = manifest();
    unknown["dispositions"][0]["name"] = json!("inferred");
    assert!(
        matches!(parse_approved_migration_manifest_json(&unknown.to_string()), Err(ManifestParseError::InvalidJson(message)) if message.contains("unknown field"))
    );

    let mut sensitive = manifest();
    sensitive["dispositions"][0]["private_key"] = json!("not allowed");
    assert!(
        matches!(parse_approved_migration_manifest_json(&sensitive.to_string()), Err(ManifestParseError::SensitiveField { path }) if path.contains("private_key"))
    );
}

#[test]
fn manifest_requires_exact_source_coverage() {
    let parsed = parse_approved_migration_manifest_json(&manifest().to_string()).unwrap();
    let source = parse_rustdesk_pro_import_json(r#"{"schema_version":1,"users":[{"user_id":"u1","username":"operator","role":null}],"groups":[],"devices":[],"address_books":[],"address_book_entries":[]}"#).unwrap();
    assert!(parsed.validate_against_source(&source).is_ok());
    let mut missing = manifest();
    missing["dispositions"][0]["source_id"] = json!("not-in-source");
    let parsed = parse_approved_migration_manifest_json(&missing.to_string()).unwrap();
    assert_eq!(
        parsed.validate_against_source(&source),
        Err(ManifestSourceCoverageError::Incomplete)
    );
}

#[test]
fn manifest_preconditions_reject_future_approval() {
    let source_json = r#"{"schema_version":1,"users":[{"user_id":"u1","username":"operator","role":null}],"groups":[],"devices":[],"address_books":[],"address_book_entries":[]}"#;
    let source = parse_rustdesk_pro_import_json(source_json).unwrap();
    let report = dry_run_import(&source, &MigrationSnapshot::default());
    let report_json = serde_json::to_vec(&report).unwrap();
    let mut value = manifest();
    value["approved_at"] = json!("2027-01-01T00:00:00Z");
    value["expires_at"] = json!("2027-02-01T00:00:00Z");
    value["source_snapshot_sha256"] = json!(sha256_digest(source_json.as_bytes()));
    value["dry_run_report_sha256"] = json!(sha256_digest(&report_json));
    value["expected_counts"] = serde_json::to_value(expected_import_counts(&source)).unwrap();
    let parsed = parse_approved_migration_manifest_json(&value.to_string()).unwrap();
    assert_eq!(
        validate_manifest_preconditions(
            &parsed,
            source_json.as_bytes(),
            &source,
            &report_json,
            time::OffsetDateTime::parse(
                "2026-12-31T23:59:59Z",
                &time::format_description::well_known::Rfc3339,
            )
            .unwrap(),
        ),
        Err(ManifestPreconditionError::NotYetApproved)
    );
}
#[test]
fn source_edges_require_unique_declared_groups_and_duplicate_ids_are_rejected() {
    let invalid = r#"{"schema_version":1,"users":[],"groups":[{"group_id":"g","name":"G"}],"devices":[],"address_books":[],"address_book_entries":[],"cross_group_edges":[{"edge_id":"e","source_group_id":"g","target_group_id":"missing"}]}"#;
    assert!(parse_rustdesk_pro_import_json(invalid).is_err());
    let self_edge = invalid.replace("missing", "g");
    assert!(parse_rustdesk_pro_import_json(&self_edge).is_err());
    let duplicate_user = r#"{"schema_version":1,"users":[{"user_id":"u","username":"a","role":null},{"user_id":"u","username":"b","role":null}],"groups":[],"devices":[],"address_books":[],"address_book_entries":[]}"#;
    assert!(parse_rustdesk_pro_import_json(duplicate_user).is_err());
}

#[test]
fn structured_entry_identity_rejects_colon_collision_and_missing_parent() {
    let mut value = manifest();
    value["dispositions"][0]["source_kind"] = json!("address_book_entry");
    value["dispositions"][0]["source_parent_id"] = json!("book:a");
    value["dispositions"][0]["source_id"] = json!("b");
    assert!(parse_approved_migration_manifest_json(&value.to_string()).is_err());
    value["dispositions"][0]["source_parent_id"] = serde_json::Value::Null;
    assert!(parse_approved_migration_manifest_json(&value.to_string()).is_err());
}

#[test]
fn wrong_kind_target_is_rejected_after_report_semantics() {
    let mut value = manifest();
    let user_uuid = "00000000-0000-0000-0000-000000000001";
    let wrong_device_uuid = "00000000-0000-0000-0000-000000000002";
    value["dispositions"][0]["target_uuid"] = json!(wrong_device_uuid);
    let parsed = parse_approved_migration_manifest_json(&value.to_string()).unwrap();
    let snapshot = MigrationSnapshot {
        users: vec![User {
            user_uuid: uuid::Uuid::parse_str(user_uuid).unwrap(),
            username: "operator".into(),
            role: "admin".into(),
        }],
        devices: vec![opendesk::domain::device::Device {
            device_uuid: uuid::Uuid::parse_str(wrong_device_uuid).unwrap(),
            rustdesk_id: Some("device".into()),
            alias: "device".into(),
            hostname: None,
            os_family: None,
            os_version: None,
            architecture: None,
            rustdesk_version: None,
            site_uuid: None,
            owner: None,
            notes: None,
            archived: false,
            last_checkin_at: None,
        }],
        ..Default::default()
    };
    let document = parse_rustdesk_pro_import_json(r#"{"schema_version":1,"users":[{"user_id":"u1","username":"operator","role":null}],"groups":[],"devices":[],"address_books":[],"address_book_entries":[]}"#).unwrap();
    let report = dry_run_import(&document, &snapshot);
    assert_eq!(
        parsed.validate_against_snapshot_and_report(&snapshot, &report),
        Err(ManifestSemanticError::InvalidTarget)
    );
}

#[test]
fn retire_allows_null_but_forbids_target_uuid() {
    let mut value = manifest();
    value["dispositions"][0]["action"] = json!("retire");
    value["dispositions"][0]["target_uuid"] = serde_json::Value::Null;
    assert!(parse_approved_migration_manifest_json(&value.to_string()).is_ok());
    value["dispositions"][0]["target_uuid"] = json!("00000000-0000-0000-0000-000000000001");
    assert!(parse_approved_migration_manifest_json(&value.to_string()).is_err());
}
