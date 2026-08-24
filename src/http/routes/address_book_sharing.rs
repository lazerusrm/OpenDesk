use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    Form,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;
use uuid::Uuid;

use super::repository_error_response;
use crate::app_state::AppState;
use crate::domain::access_policy::Action;
use crate::http::session::{require_action, require_csrf};
use crate::http::views::{
    AddressBookAccessRuleView, AddressBookShareOptionView, SharedAddressBookRowView,
};
use crate::repository::access_groups::list_access_groups;
use crate::repository::address_books::{
    delete_address_book_access_rule, find_address_book_for_owner, list_address_book_access_rules,
    list_shared_address_books_for_user, upsert_address_book_access_rule,
    AddressBookRepositoryError,
};
use crate::repository::users::list_users;

#[derive(Deserialize)]
pub(super) struct AddressBookAccessRuleForm {
    pub csrf_token: String,
    pub principal_type: String,
    pub principal_uuid: String,
    pub permission: String,
}

#[derive(Deserialize)]
pub(super) struct AddressBookAccessRuleDeleteForm {
    pub csrf_token: String,
}

pub(super) struct OwnedBookShareView {
    pub can_share: bool,
    pub access_rules: Vec<AddressBookAccessRuleView>,
    pub user_share_options: Vec<AddressBookShareOptionView>,
    pub group_share_options: Vec<AddressBookShareOptionView>,
}

fn permission_display(permission: &str) -> String {
    match permission {
        "read" => "Read".to_string(),
        "write" => "Write".to_string(),
        "admin" => "Admin".to_string(),
        other => other.to_string(),
    }
}

pub(super) async fn load_owned_book_share_view(
    state: &AppState,
    owner_user_uuid: Uuid,
    address_book_uuid: Uuid,
    kind_display: &str,
    can_update: bool,
) -> Result<OwnedBookShareView, Response> {
    let can_share = can_update && kind_display == "Shared";
    if !can_share {
        return Ok(OwnedBookShareView {
            can_share: false,
            access_rules: Vec::new(),
            user_share_options: Vec::new(),
            group_share_options: Vec::new(),
        });
    }
    let rules =
        match list_address_book_access_rules(&state.db, owner_user_uuid, address_book_uuid).await {
            Ok(rules) => rules,
            Err(AddressBookRepositoryError::Forbidden | AddressBookRepositoryError::NotFound) => {
                return Ok(OwnedBookShareView {
                    can_share: false,
                    access_rules: Vec::new(),
                    user_share_options: Vec::new(),
                    group_share_options: Vec::new(),
                })
            }
            Err(error) => return Err(repository_error_response(error)),
        };
    let users = list_users(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let groups = list_access_groups(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let user_names: HashMap<Uuid, String> = users
        .iter()
        .map(|user| (user.user_uuid, user.username.clone()))
        .collect();
    let group_names: HashMap<Uuid, String> = groups
        .iter()
        .map(|group| (group.access_group_uuid, group.name.clone()))
        .collect();
    Ok(OwnedBookShareView {
        can_share: true,
        access_rules: rules
            .into_iter()
            .map(|rule| {
                let principal_label = match rule.principal_type.as_str() {
                    "user" => user_names
                        .get(&rule.principal_uuid)
                        .cloned()
                        .unwrap_or_else(|| rule.principal_uuid.to_string()),
                    "group" => group_names
                        .get(&rule.principal_uuid)
                        .cloned()
                        .unwrap_or_else(|| rule.principal_uuid.to_string()),
                    _ => rule.principal_uuid.to_string(),
                };
                AddressBookAccessRuleView {
                    principal_type_display: if rule.principal_type == "group" {
                        "Group".to_string()
                    } else {
                        "User".to_string()
                    },
                    permission_display: permission_display(&rule.permission),
                    principal_type: rule.principal_type,
                    principal_uuid: rule.principal_uuid.to_string(),
                    principal_label,
                    permission: rule.permission,
                }
            })
            .collect(),
        user_share_options: users
            .into_iter()
            .filter(|user| user.user_uuid != owner_user_uuid)
            .map(|user| AddressBookShareOptionView {
                principal_uuid: user.user_uuid.to_string(),
                label: user.username,
            })
            .collect(),
        group_share_options: groups
            .into_iter()
            .map(|group| AddressBookShareOptionView {
                principal_uuid: group.access_group_uuid.to_string(),
                label: group.name,
            })
            .collect(),
    })
}

pub(super) async fn load_shared_with_me(
    state: &AppState,
    user_uuid: Uuid,
) -> Result<Vec<SharedAddressBookRowView>, Response> {
    let books = list_shared_address_books_for_user(&state.db, user_uuid)
        .await
        .map_err(repository_error_response)?;
    Ok(books
        .into_iter()
        .filter(|(book, _, _)| book.owner_user_uuid != user_uuid)
        .map(|(book, owner, permission)| SharedAddressBookRowView {
            name: book.name,
            owner,
            permission_display: permission_display(&permission),
        })
        .collect())
}

pub(super) async fn address_book_access_rule_create_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(address_book_uuid): Path<Uuid>,
    Form(form): Form<AddressBookAccessRuleForm>,
) -> Result<Response, Response> {
    let actor = require_action(&state, &jar, Action::AddressBookUpdate).await?;
    find_address_book_for_owner(&state.db, actor.user_uuid, address_book_uuid)
        .await
        .map_err(repository_error_response)?;
    require_csrf(&actor, &form.csrf_token)?;
    let principal_uuid = Uuid::parse_str(form.principal_uuid.trim())
        .map_err(|_| StatusCode::BAD_REQUEST.into_response())?;
    upsert_address_book_access_rule(
        &state.db,
        actor.user_uuid,
        address_book_uuid,
        form.principal_type.trim(),
        principal_uuid,
        form.permission.trim(),
    )
    .await
    .map_err(repository_error_response)?;
    Ok(Redirect::to(&format!("/address-books/{address_book_uuid}")).into_response())
}

pub(super) async fn address_book_access_rule_delete_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    Path((address_book_uuid, principal_type, principal_uuid)): Path<(Uuid, String, Uuid)>,
    Form(form): Form<AddressBookAccessRuleDeleteForm>,
) -> Result<Response, Response> {
    let actor = require_action(&state, &jar, Action::AddressBookDelete).await?;
    find_address_book_for_owner(&state.db, actor.user_uuid, address_book_uuid)
        .await
        .map_err(repository_error_response)?;
    require_csrf(&actor, &form.csrf_token)?;
    delete_address_book_access_rule(
        &state.db,
        actor.user_uuid,
        address_book_uuid,
        principal_type.trim(),
        principal_uuid,
    )
    .await
    .map_err(repository_error_response)?;
    Ok(Redirect::to(&format!("/address-books/{address_book_uuid}")).into_response())
}
