use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Membership grants an operator user access to an access group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccessGroupMembership {
    pub access_group_uuid: Uuid,
    pub user_uuid: Uuid,
}
