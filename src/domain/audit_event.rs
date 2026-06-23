use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEventDraft {
    pub actor_user_uuid: Option<Uuid>,
    pub action: String,
    pub object_type: String,
    pub object_uuid: Option<Uuid>,
    pub outcome: String,
    pub source: String,
    pub detail: Option<Value>,
}

pub fn redact_audit_detail(detail: &Value) -> Value {
    match detail {
        Value::Object(map) => {
            let mut redacted = serde_json::Map::new();
            for (key, value) in map {
                let lower = key.to_ascii_lowercase();
                if lower.contains("password")
                    || lower.contains("secret")
                    || lower.contains("token")
                    || lower.contains("key")
                {
                    redacted.insert(key.clone(), Value::String("[redacted]".to_string()));
                } else {
                    redacted.insert(key.clone(), redact_audit_detail(value));
                }
            }
            Value::Object(redacted)
        }
        Value::Array(items) => Value::Array(items.iter().map(redact_audit_detail).collect()),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn redact_audit_detail_masks_sensitive_keys() {
        let detail = json!({
            "username": "admin",
            "enrollment_token": "secret-value",
            "nested": {"password_hash": "abc"}
        });
        let redacted = redact_audit_detail(&detail);
        assert_eq!(redacted["username"], "admin");
        assert_eq!(redacted["enrollment_token"], "[redacted]");
        assert_eq!(redacted["nested"]["password_hash"], "[redacted]");
    }
}