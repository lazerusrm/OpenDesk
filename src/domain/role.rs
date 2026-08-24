use thiserror::Error;

/// Operator account role stored on `users.role`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Admin,
    Operator,
    ReadOnly,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RoleParseError {
    #[error("unknown role")]
    Unknown,
}

impl Role {
    pub const ADMIN: &'static str = "admin";
    pub const OPERATOR: &'static str = "operator";
    pub const READ_ONLY: &'static str = "read_only";

    pub fn parse(value: &str) -> Result<Self, RoleParseError> {
        match value {
            Self::ADMIN => Ok(Self::Admin),
            Self::OPERATOR => Ok(Self::Operator),
            Self::READ_ONLY => Ok(Self::ReadOnly),
            _ => Err(RoleParseError::Unknown),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Admin => Self::ADMIN,
            Self::Operator => Self::OPERATOR,
            Self::ReadOnly => Self::READ_ONLY,
        }
    }

    pub fn display_label(self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::Operator => "operator",
            Self::ReadOnly => "read-only",
        }
    }

    /// Any authenticated role may view dashboard pages and audit history.
    pub fn can_read(self) -> bool {
        true
    }

    /// Admins and operators may perform device/inventory mutations and exports.
    pub fn can_mutate(self) -> bool {
        matches!(self, Self::Admin | Self::Operator)
    }

    /// Only admins may manage users, restore backups, and edit server config.
    pub fn can_admin(self) -> bool {
        matches!(self, Self::Admin)
    }
}

pub fn validate_role_value(value: &str) -> Result<Role, RoleParseError> {
    Role::parse(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_canonical_roles() {
        assert_eq!(Role::parse("admin"), Ok(Role::Admin));
        assert_eq!(Role::parse("operator"), Ok(Role::Operator));
        assert_eq!(Role::parse("read_only"), Ok(Role::ReadOnly));
    }

    #[test]
    fn parse_rejects_unknown() {
        for value in [
            "superuser",
            "read-only",
            "readonly",
            " read_only",
            "read_only ",
        ] {
            assert_eq!(Role::parse(value), Err(RoleParseError::Unknown));
        }
    }

    #[test]
    fn permission_matrix() {
        assert!(Role::ReadOnly.can_read());
        assert!(!Role::ReadOnly.can_mutate());
        assert!(!Role::ReadOnly.can_admin());
        assert!(Role::Operator.can_mutate());
        assert!(!Role::Operator.can_admin());
        assert!(Role::Admin.can_admin());
        assert!(Role::Admin.can_mutate());
    }
}
