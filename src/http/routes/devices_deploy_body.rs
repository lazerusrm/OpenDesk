use serde_json::{Map, Value};

const MAX_ID_LEN: usize = 64;
const MAX_UUID_LEN: usize = 128;
const MAX_PK_LEN: usize = 1024;
const MAX_ALIAS_LEN: usize = 128;
const MAX_NOTE_LEN: usize = 1024;
const MAX_HOSTNAME_LEN: usize = 128;

pub(crate) fn parse_deploy_body(body: &Value) -> Option<(String, Option<String>)> {
    let map = body.as_object()?;
    let rustdesk_id = require_text(map, "id", MAX_ID_LEN).ok()?;
    let _uuid = require_text(map, "uuid", MAX_UUID_LEN).ok()?;
    let _pk = require_text(map, "pk", MAX_PK_LEN).ok()?;
    let hostname = json_text(map, "hostname", MAX_HOSTNAME_LEN).ok()?;
    Some((rustdesk_id, hostname))
}

pub(crate) fn parse_cli_body(body: &Value) -> Option<(String, Option<String>, Option<String>)> {
    let map = body.as_object()?;
    let rustdesk_id = require_text(map, "id", MAX_ID_LEN).ok()?;
    let _uuid = require_text(map, "uuid", MAX_UUID_LEN).ok()?;
    let device_name = json_text(map, "device_name", MAX_ALIAS_LEN).ok()?;
    let note = json_text(map, "note", MAX_NOTE_LEN).ok()?;
    Some((rustdesk_id, device_name, note))
}

fn json_text(map: &Map<String, Value>, key: &str, max_len: usize) -> Result<Option<String>, ()> {
    match map.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                Ok(None)
            } else if trimmed.len() > max_len || trimmed.chars().any(char::is_control) {
                Err(())
            } else {
                Ok(Some(trimmed.to_string()))
            }
        }
        Some(_) => Err(()),
    }
}

fn require_text(map: &Map<String, Value>, key: &str, max_len: usize) -> Result<String, ()> {
    json_text(map, key, max_len)?.ok_or(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_deploy_body_accepts_extra_fields() {
        let parsed = parse_deploy_body(&json!({
            "id": "123456789",
            "uuid": "client-uuid",
            "pk": "cGs=",
            "hostname": "ws-01",
            "unexpected": true
        }))
        .expect("parsed");
        assert_eq!(parsed.0, "123456789");
        assert_eq!(parsed.1.as_deref(), Some("ws-01"));
    }

    #[test]
    fn parse_deploy_body_rejects_empty_id() {
        assert!(parse_deploy_body(&json!({
            "id": "  ",
            "uuid": "client-uuid",
            "pk": "cGs="
        }))
        .is_none());
    }

    #[test]
    fn parse_cli_body_ignores_address_book_password() {
        let parsed = parse_cli_body(&json!({
            "id": "123456789",
            "uuid": "client-uuid",
            "device_name": "lab-pc",
            "note": "shelf-a",
            "address_book_password": "must-not-be-read"
        }))
        .expect("parsed");
        assert_eq!(parsed.0, "123456789");
        assert_eq!(parsed.1.as_deref(), Some("lab-pc"));
        assert_eq!(parsed.2.as_deref(), Some("shelf-a"));
    }
}
