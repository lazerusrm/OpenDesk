use super::role::Role;
use thiserror::Error;

/// Canonical dashboard/API request operations.
///
/// These values authorize signed-in OpenDesk requests only. Authentication and
/// persisted audit-event names are separate concerns and are intentionally not
/// members of this allowlist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    DeviceList,
    DeviceView,
    DeviceCreate,
    DeviceUpdate,
    DeviceArchive,
    DeviceUnarchive,
    DeviceExport,
    SiteList,
    SiteView,
    SiteCreate,
    SiteUpdate,
    SiteDelete,
    TagList,
    TagView,
    TagCreate,
    TagUpdate,
    TagDelete,
    EnrollmentTokenList,
    EnrollmentTokenCreate,
    EnrollmentTokenRevoke,
    DeploymentView,
    DeploymentScriptExport,
    AuditView,
    AuditExport,
    BackupView,
    BackupExport,
    BackupRestore,
    UserList,
    UserCreate,
    ServerConfigView,
    ServerConfigUpdate,
    StatusView,
    AddressBookList,
    AddressBookView,
    AddressBookCreate,
    AddressBookUpdate,
    AddressBookDelete,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("unknown dashboard/API action")]
pub struct ActionParseError;

impl Action {
    pub const ALL: &'static [Self] = &[
        Self::DeviceList,
        Self::DeviceView,
        Self::DeviceCreate,
        Self::DeviceUpdate,
        Self::DeviceArchive,
        Self::DeviceUnarchive,
        Self::DeviceExport,
        Self::SiteList,
        Self::SiteView,
        Self::SiteCreate,
        Self::SiteUpdate,
        Self::SiteDelete,
        Self::TagList,
        Self::TagView,
        Self::TagCreate,
        Self::TagUpdate,
        Self::TagDelete,
        Self::EnrollmentTokenList,
        Self::EnrollmentTokenCreate,
        Self::EnrollmentTokenRevoke,
        Self::DeploymentView,
        Self::DeploymentScriptExport,
        Self::AuditView,
        Self::AuditExport,
        Self::BackupView,
        Self::BackupExport,
        Self::BackupRestore,
        Self::UserList,
        Self::UserCreate,
        Self::ServerConfigView,
        Self::ServerConfigUpdate,
        Self::StatusView,
        Self::AddressBookList,
        Self::AddressBookView,
        Self::AddressBookCreate,
        Self::AddressBookUpdate,
        Self::AddressBookDelete,
    ];

    /// Parse one exact canonical action. Unknown and noncanonical values fail
    /// through the same boundary; no aliases or normalization are accepted.
    pub fn parse(value: &str) -> Result<Self, ActionParseError> {
        Self::ALL
            .iter()
            .copied()
            .find(|action| action.as_str() == value)
            .ok_or(ActionParseError)
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DeviceList => "device_list",
            Self::DeviceView => "device_view",
            Self::DeviceCreate => "device_create",
            Self::DeviceUpdate => "device_update",
            Self::DeviceArchive => "device_archive",
            Self::DeviceUnarchive => "device_unarchive",
            Self::DeviceExport => "device_export",
            Self::SiteList => "site_list",
            Self::SiteView => "site_view",
            Self::SiteCreate => "site_create",
            Self::SiteUpdate => "site_update",
            Self::SiteDelete => "site_delete",
            Self::TagList => "tag_list",
            Self::TagView => "tag_view",
            Self::TagCreate => "tag_create",
            Self::TagUpdate => "tag_update",
            Self::TagDelete => "tag_delete",
            Self::EnrollmentTokenList => "enrollment_token_list",
            Self::EnrollmentTokenCreate => "enrollment_token_create",
            Self::EnrollmentTokenRevoke => "enrollment_token_revoke",
            Self::DeploymentView => "deployment_view",
            Self::DeploymentScriptExport => "deployment_script_export",
            Self::AuditView => "audit_view",
            Self::AuditExport => "audit_export",
            Self::BackupView => "backup_view",
            Self::BackupExport => "backup_export",
            Self::BackupRestore => "backup_restore",
            Self::UserList => "user_list",
            Self::UserCreate => "user_create",
            Self::ServerConfigView => "server_config_view",
            Self::ServerConfigUpdate => "server_config_update",
            Self::StatusView => "status_view",
            Self::AddressBookList => "address_book_list",
            Self::AddressBookView => "address_book_view",
            Self::AddressBookCreate => "address_book_create",
            Self::AddressBookUpdate => "address_book_update",
            Self::AddressBookDelete => "address_book_delete",
        }
    }

    /// Whether this operation must be paired with the separate device
    /// visibility predicate before reading or changing a device.
    pub const fn requires_device_visibility(self) -> bool {
        matches!(
            self,
            Self::DeviceList
                | Self::DeviceView
                | Self::DeviceUpdate
                | Self::DeviceArchive
                | Self::DeviceUnarchive
                | Self::DeviceExport
                | Self::AddressBookView
                | Self::AddressBookUpdate
                | Self::AddressBookDelete
        )
    }

    /// Global role authorization. This does not inspect device visibility and
    /// does not authorize or deny a RustDesk session.
    pub const fn allowed_for(self, role: Role) -> bool {
        match role {
            Role::Admin => true,
            Role::Operator => matches!(
                self,
                Self::DeviceList
                    | Self::DeviceView
                    | Self::DeviceCreate
                    | Self::DeviceUpdate
                    | Self::DeviceArchive
                    | Self::DeviceUnarchive
                    | Self::DeviceExport
                    | Self::SiteList
                    | Self::SiteView
                    | Self::SiteCreate
                    | Self::SiteUpdate
                    | Self::SiteDelete
                    | Self::TagList
                    | Self::TagView
                    | Self::TagCreate
                    | Self::TagUpdate
                    | Self::TagDelete
                    | Self::EnrollmentTokenList
                    | Self::EnrollmentTokenCreate
                    | Self::EnrollmentTokenRevoke
                    | Self::DeploymentView
                    | Self::DeploymentScriptExport
                    | Self::AuditView
                    | Self::AuditExport
                    | Self::BackupView
                    | Self::StatusView
                    | Self::AddressBookList
                    | Self::AddressBookView
                    | Self::AddressBookCreate
                    | Self::AddressBookUpdate
                    | Self::AddressBookDelete
            ),
            Role::ReadOnly => matches!(
                self,
                Self::DeviceList
                    | Self::DeviceView
                    | Self::SiteList
                    | Self::SiteView
                    | Self::TagList
                    | Self::TagView
                    | Self::EnrollmentTokenList
                    | Self::DeploymentView
                    | Self::AuditView
                    | Self::BackupView
                    | Self::StatusView
                    | Self::AddressBookList
                    | Self::AddressBookView
            ),
        }
    }
}

/// Canonical role/action authorization predicate for signed-in requests.
pub const fn is_action_allowed(role: Role, action: Action) -> bool {
    action.allowed_for(role)
}

/// Parse exact stored role and action values, denying either unknown value.
/// This boundary deliberately does not use the broader role display parser,
/// which has compatibility input handling for existing user records.
pub fn is_action_allowed_for_values(role: &str, action: &str) -> bool {
    let role = match role {
        Role::ADMIN => Role::Admin,
        Role::OPERATOR => Role::Operator,
        Role::READ_ONLY => Role::ReadOnly,
        _ => return false,
    };
    let Ok(action) = Action::parse(action) else {
        return false;
    };
    is_action_allowed(role, action)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_allowlisted_action_round_trips_and_unknowns_fail_closed() {
        assert_eq!(Action::ALL.len(), 37);
        for action in Action::ALL {
            assert_eq!(Action::parse(action.as_str()), Ok(*action));
        }
        for value in [
            "",
            " dashboard_read",
            "device-read",
            "login",
            "endpoint_checkin",
            "audit_event",
        ] {
            assert_eq!(Action::parse(value), Err(ActionParseError));
        }
    }

    #[test]
    fn role_action_matrix_is_exhaustive() {
        for action in Action::ALL {
            assert!(action.allowed_for(Role::Admin));
            assert_eq!(
                action.allowed_for(Role::Operator),
                matches!(
                    action,
                    Action::DeviceList
                        | Action::DeviceView
                        | Action::DeviceCreate
                        | Action::DeviceUpdate
                        | Action::DeviceArchive
                        | Action::DeviceUnarchive
                        | Action::DeviceExport
                        | Action::SiteList
                        | Action::SiteView
                        | Action::SiteCreate
                        | Action::SiteUpdate
                        | Action::SiteDelete
                        | Action::TagList
                        | Action::TagView
                        | Action::TagCreate
                        | Action::TagUpdate
                        | Action::TagDelete
                        | Action::EnrollmentTokenList
                        | Action::EnrollmentTokenCreate
                        | Action::EnrollmentTokenRevoke
                        | Action::DeploymentView
                        | Action::DeploymentScriptExport
                        | Action::AuditView
                        | Action::AuditExport
                        | Action::BackupView
                        | Action::StatusView
                        | Action::AddressBookList
                        | Action::AddressBookView
                        | Action::AddressBookCreate
                        | Action::AddressBookUpdate
                        | Action::AddressBookDelete
                )
            );
            assert_eq!(
                action.allowed_for(Role::ReadOnly),
                matches!(
                    action,
                    Action::DeviceList
                        | Action::DeviceView
                        | Action::SiteList
                        | Action::SiteView
                        | Action::TagList
                        | Action::TagView
                        | Action::EnrollmentTokenList
                        | Action::DeploymentView
                        | Action::AuditView
                        | Action::BackupView
                        | Action::StatusView
                        | Action::AddressBookList
                        | Action::AddressBookView
                )
            );
        }
    }

    #[test]
    fn value_boundary_denies_unknown_role_and_action() {
        assert!(is_action_allowed_for_values("admin", "device_view"));
        assert!(!is_action_allowed_for_values("read-only", "device_view"));
        assert!(!is_action_allowed_for_values("unknown", "device_view"));
        assert!(!is_action_allowed_for_values("admin", "device-view"));
    }
    #[test]
    fn only_existing_device_actions_need_visibility() {
        for action in Action::ALL {
            assert_eq!(
                action.requires_device_visibility(),
                matches!(
                    action,
                    Action::DeviceList
                        | Action::DeviceView
                        | Action::DeviceUpdate
                        | Action::DeviceArchive
                        | Action::DeviceUnarchive
                        | Action::DeviceExport
                        | Action::AddressBookView
                        | Action::AddressBookUpdate
                        | Action::AddressBookDelete
                )
            );
        }
    }
}
