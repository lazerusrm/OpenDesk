mod common;

use common::test_state;
use opendesk::domain::backup::{
    parse_backup_json, render_backup_json, BackupDocument, BACKUP_SCHEMA_VERSION,
};
use opendesk::repository::backup::{export_backup_document, restore_backup_document};

#[tokio::test]
async fn backup_v2_unknown_fields_are_rejected_at_every_owned_boundary() {
    let state = test_state().await;
    let current = export_backup_document(&state.db).await.expect("export");
    let mut top_level = serde_json::to_value(&current).expect("serialize current");
    top_level
        .as_object_mut()
        .expect("object")
        .insert("unexpected".to_string(), serde_json::json!(true));
    assert!(parse_backup_json(&serde_json::to_string(&top_level).expect("json")).is_err());

    let mut nested = serde_json::to_value(&current).expect("serialize current");
    nested["sensitivity"] = serde_json::json!({
        "contains_password_hashes": true,
        "contains_enrollment_token_hashes": true,
        "excludes_sessions": true,
        "excludes_audit_events": true,
        "excludes_endpoint_checkins": true,
        "unexpected": true
    });
    assert!(parse_backup_json(&serde_json::to_string(&nested).expect("json")).is_err());
}

#[tokio::test]
async fn backup_v2_rejects_unknown_fields_in_existing_nested_dtos() {
    let state = test_state().await;
    let current = export_backup_document(&state.db).await.expect("export");
    let cases = [
        (
            "sites",
            serde_json::json!([{
                "site_uuid": uuid::Uuid::new_v4(),
                "name": "Site",
                "unexpected": true
            }]),
        ),
        (
            "tags",
            serde_json::json!([{
                "tag_uuid": uuid::Uuid::new_v4(),
                "name": "Tag",
                "unexpected": true
            }]),
        ),
        (
            "devices",
            serde_json::json!([{
                "device_uuid": uuid::Uuid::new_v4(),
                "rustdesk_id": null,
                "alias": "Device",
                "hostname": null,
                "os_family": null,
                "os_version": null,
                "architecture": null,
                "rustdesk_version": null,
                "site_uuid": null,
                "owner": null,
                "notes": null,
                "archived": false,
                "last_checkin_at": null,
                "unexpected": true
            }]),
        ),
        (
            "server_config",
            serde_json::json!({
                "id_server": "rd.example.com",
                "relay_server": "rd.example.com",
                "api_server": "",
                "public_key": "",
                "unexpected": true
            }),
        ),
    ];
    for (field, value) in cases {
        let mut document = serde_json::to_value(&current).expect("serialize current");
        document[field] = value;
        assert!(
            parse_backup_json(&serde_json::to_string(&document).expect("json")).is_err(),
            "unknown field in {field} should be rejected"
        );
    }
}

#[tokio::test]
async fn backup_v1_boundary_conversion_restores_without_new_collections() {
    let source = test_state().await;
    let current = export_backup_document(&source.db).await.expect("export");
    let mut v1 = serde_json::to_value(&current).expect("serialize current");
    let object = v1.as_object_mut().expect("object");
    object.insert("schema_version".to_string(), serde_json::json!(1));
    for field in [
        "access_groups",
        "access_group_memberships",
        "device_visibility_grants",
        "user_device_visibility_grants",
        "access_group_access_grants",
        "address_books",
        "address_book_access_rules",
        "address_book_tags",
        "address_book_entries",
        "address_book_entry_tags",
    ] {
        object.remove(field);
    }
    for user in object["users"].as_array_mut().expect("users") {
        user.as_object_mut()
            .expect("user")
            .remove("activation_state");
    }
    let parsed =
        parse_backup_json(&serde_json::to_string(&v1).expect("serialize v1")).expect("convert v1");
    assert_eq!(parsed.schema_version, BACKUP_SCHEMA_VERSION);
    assert!(parsed.access_groups.is_empty());

    let target = test_state().await;
    restore_backup_document(&target.db, &parsed)
        .await
        .expect("restore converted v1");
    let restored = export_backup_document(&target.db)
        .await
        .expect("export restored");
    assert_eq!(restored.schema_version, BACKUP_SCHEMA_VERSION);
    assert!(restored.access_groups.is_empty());
    assert!(restored.address_books.is_empty());

    let round_trip: BackupDocument =
        parse_backup_json(&render_backup_json(&restored).expect("render restored"))
            .expect("parse restored v2");
    assert_eq!(round_trip, restored);
}
