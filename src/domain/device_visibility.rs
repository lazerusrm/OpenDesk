use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Grants every member of an access group visibility of a device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceVisibilityGrant {
    pub access_group_uuid: Uuid,
    pub device_uuid: Uuid,
}

/// Grants one user visibility of one device directly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserDeviceVisibilityGrant {
    pub user_uuid: Uuid,
    pub device_uuid: Uuid,
}
