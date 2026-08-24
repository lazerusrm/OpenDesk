use super::NavPermissions;
use askama::Template;

#[derive(Template)]
#[template(path = "access_groups.html")]
pub struct AccessGroupsListView {
    pub title: String,
    pub show_nav: bool,
    pub nav: NavPermissions,
    pub csrf_token: String,
    pub groups: Vec<AccessGroupRowView>,
    pub show_all_groups: bool,
    pub show_user_memberships: bool,
    pub show_device_visibility: bool,
    pub error_message: Option<String>,
}

#[derive(Clone)]
pub struct AccessGroupRowView {
    pub access_group_uuid: String,
    pub name: String,
    pub member_count: usize,
    pub device_count: usize,
    pub member_count_label: String,
    pub device_count_label: String,
    pub member_names_display: String,
    pub device_names_display: String,
}

#[derive(Template)]
#[template(path = "access_group_detail.html")]
pub struct AccessGroupDetailView {
    pub title: String,
    pub show_nav: bool,
    pub nav: NavPermissions,
    pub csrf_token: String,
    pub access_group_uuid: String,
    pub name: String,
    pub members: Vec<AccessGroupMemberView>,
    pub devices: Vec<AccessGroupDeviceView>,
    pub member_count_label: String,
    pub device_count_label: String,
    pub user_options: Vec<AccessGroupUserOptionView>,
    pub device_options: Vec<AccessGroupDeviceOptionView>,
    pub group_options: Vec<AccessGroupGroupOptionView>,
    pub incoming_group_names_display: String,
    pub error_message: Option<String>,
}

#[derive(Clone)]
pub struct AccessGroupMemberView {
    pub username: String,
    pub user_uuid: String,
}

#[derive(Clone)]
pub struct AccessGroupDeviceView {
    pub alias: String,
    pub rustdesk_id: String,
    pub device_uuid: String,
}

#[derive(Clone)]
pub struct AccessGroupUserOptionView {
    pub username: String,
    pub user_uuid: String,
    pub selected: bool,
}

#[derive(Clone)]
pub struct AccessGroupDeviceOptionView {
    pub alias: String,
    pub rustdesk_id: String,
    pub device_uuid: String,
    pub selected: bool,
}

#[derive(Clone)]
pub struct AccessGroupGroupOptionView {
    pub name: String,
    pub access_group_uuid: String,
    pub selected: bool,
}

#[derive(Template)]
#[template(path = "address_books.html")]
pub struct AddressBooksListView {
    pub title: String,
    pub show_nav: bool,
    pub nav: NavPermissions,
    pub can_create: bool,
    pub can_update: bool,
    pub can_delete: bool,
    pub csrf_token: String,
    pub books: Vec<AddressBookRowView>,
    pub shared_books: Vec<SharedAddressBookRowView>,
    pub error_message: Option<String>,
}

#[derive(Clone)]
pub struct AddressBookRowView {
    pub address_book_uuid: String,
    pub name: String,
    pub kind_display: String,
}

#[derive(Clone)]
pub struct SharedAddressBookRowView {
    pub name: String,
    pub owner: String,
    pub permission_display: String,
}

#[derive(Template)]
#[template(path = "address_book_detail.html")]
pub struct AddressBookDetailView {
    pub title: String,
    pub show_nav: bool,
    pub nav: NavPermissions,
    pub can_create: bool,
    pub can_update: bool,
    pub can_delete: bool,
    pub csrf_token: String,
    pub address_book_uuid: String,
    pub name: String,
    pub kind_display: String,
    pub entries: Vec<AddressBookEntryRowView>,
    pub device_options: Vec<AddressBookDeviceOptionView>,
    pub can_share: bool,
    pub access_rules: Vec<AddressBookAccessRuleView>,
    pub user_share_options: Vec<AddressBookShareOptionView>,
    pub group_share_options: Vec<AddressBookShareOptionView>,
    pub error_message: Option<String>,
}

#[derive(Clone)]
pub struct AddressBookEntryRowView {
    pub address_book_entry_uuid: String,
    pub device_uuid: String,
    pub device_alias: String,
    pub alias: String,
    pub notes: String,
    pub notes_display: String,
    pub notes_title: String,
    pub position: u32,
    pub rustdesk_id_display: String,
    pub rustdesk_id_copy_text: String,
}

#[derive(Clone)]
pub struct AddressBookDeviceOptionView {
    pub device_uuid: String,
    pub alias: String,
    pub rustdesk_id: String,
}

#[derive(Clone)]
pub struct AddressBookAccessRuleView {
    pub principal_type: String,
    pub principal_type_display: String,
    pub principal_uuid: String,
    pub principal_label: String,
    pub permission: String,
    pub permission_display: String,
}

#[derive(Clone)]
pub struct AddressBookShareOptionView {
    pub principal_uuid: String,
    pub label: String,
}
