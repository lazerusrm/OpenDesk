use std::collections::BTreeSet;
use uuid::Uuid;

/// Facts assembled by the access-grant boundary for one user/device request.
///
/// `access_group_uuids` is the user's effective `AccessGroupMembership` set;
/// `device_access_group_uuids` is the device's `DeviceVisibilityGrant` set;
/// `directly_granted_device_uuids` contains this user's
/// `UserDeviceVisibilityGrant` targets; and `reachable_outgoing_access_group_uuids`
/// is the outgoing groups of `AccessGroupAccessGrant` rows whose incoming group
/// the user belongs to. The policy intentionally owns no persistence or
/// aggregate schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibilityFacts {
    pub user_uuid: Uuid,
    pub device_uuid: Uuid,
    pub access_group_uuids: BTreeSet<Uuid>,
    pub device_access_group_uuids: BTreeSet<Uuid>,
    pub directly_granted_device_uuids: BTreeSet<Uuid>,
    pub reachable_outgoing_access_group_uuids: BTreeSet<Uuid>,
}

impl VisibilityFacts {
    pub fn new(user_uuid: Uuid, device_uuid: Uuid) -> Self {
        Self {
            user_uuid,
            device_uuid,
            access_group_uuids: BTreeSet::new(),
            device_access_group_uuids: BTreeSet::new(),
            directly_granted_device_uuids: BTreeSet::new(),
            reachable_outgoing_access_group_uuids: BTreeSet::new(),
        }
    }
}

/// True only when an explicit direct or shared access-group grant exists.
///
/// This is device visibility for dashboard/API authorization. It does not
/// authorize or deny a RustDesk session. Address-book ownership is a separate
/// policy and must not be inferred from these facts.
pub fn is_device_visible(facts: &VisibilityFacts) -> bool {
    has_direct_device_visibility_grant(facts)
        || has_access_group_device_visibility_grant(facts)
        || has_access_group_access_visibility_grant(facts)
}

/// Explicit direct `UserDeviceVisibilityGrant` predicate.
pub fn has_direct_device_visibility_grant(facts: &VisibilityFacts) -> bool {
    facts
        .directly_granted_device_uuids
        .contains(&facts.device_uuid)
}

/// Explicit shared `AccessGroup`/`DeviceVisibilityGrant` predicate.
pub fn has_access_group_device_visibility_grant(facts: &VisibilityFacts) -> bool {
    facts
        .access_group_uuids
        .iter()
        .any(|group_uuid| facts.device_access_group_uuids.contains(group_uuid))
}

/// Explicit `AccessGroupAccessGrant` predicate: a group the user belongs to
/// may see devices granted to another group.
pub fn has_access_group_access_visibility_grant(facts: &VisibilityFacts) -> bool {
    facts
        .reachable_outgoing_access_group_uuids
        .iter()
        .any(|group_uuid| facts.device_access_group_uuids.contains(group_uuid))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids() -> (Uuid, Uuid, Uuid, Uuid) {
        (
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        )
    }

    #[test]
    fn visibility_is_default_deny_and_only_explicit_grants_allow() {
        let (user_uuid, device_uuid, user_group, device_group) = ids();
        let mut facts = VisibilityFacts::new(user_uuid, device_uuid);
        assert!(!is_device_visible(&facts));
        assert!(!has_direct_device_visibility_grant(&facts));
        assert!(!has_access_group_device_visibility_grant(&facts));

        facts.directly_granted_device_uuids.insert(device_uuid);
        assert!(is_device_visible(&facts));
        assert!(has_direct_device_visibility_grant(&facts));
        assert!(!has_access_group_device_visibility_grant(&facts));

        facts.directly_granted_device_uuids.clear();
        facts.access_group_uuids.insert(user_group);
        facts.device_access_group_uuids.insert(device_group);
        assert!(!is_device_visible(&facts));
        facts.device_access_group_uuids.insert(user_group);
        assert!(is_device_visible(&facts));
        assert!(has_access_group_device_visibility_grant(&facts));
    }

    #[test]
    fn group_access_grant_sees_outgoing_group_devices() {
        let (user_uuid, device_uuid, incoming, outgoing) = ids();
        let mut facts = VisibilityFacts::new(user_uuid, device_uuid);
        facts.access_group_uuids.insert(incoming);
        facts.device_access_group_uuids.insert(outgoing);
        assert!(!is_device_visible(&facts));
        facts.reachable_outgoing_access_group_uuids.insert(outgoing);
        assert!(is_device_visible(&facts));
        assert!(has_access_group_access_visibility_grant(&facts));
        assert!(!has_access_group_device_visibility_grant(&facts));
    }

    #[test]
    fn unrelated_group_or_device_grant_does_not_leak_visibility() {
        let (user_uuid, device_uuid, user_group, device_group) = ids();
        let mut facts = VisibilityFacts::new(user_uuid, device_uuid);
        facts.access_group_uuids.insert(user_group);
        facts.device_access_group_uuids.insert(device_group);
        facts.directly_granted_device_uuids.insert(Uuid::new_v4());
        assert!(!is_device_visible(&facts));
    }

    #[test]
    fn visibility_is_independent_of_role_site_tag_and_owner() {
        let (user_uuid, device_uuid, user_group, _) = ids();
        let mut facts = VisibilityFacts::new(user_uuid, device_uuid);
        facts.access_group_uuids.insert(user_group);
        facts.device_access_group_uuids.insert(user_group);
        assert!(is_device_visible(&facts));
        // No Role, site, tag, owner, or RustDesk session state is an input.
    }
}
