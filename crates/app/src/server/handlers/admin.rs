/// Admin endpoints — user management and basic usage analytics.
///
/// All routes require the `Admin` group.
///
/// GET  /api/admin/users             — list all Cognito users
/// PATCH /api/admin/users/:id/groups — update group membership
/// GET  /api/admin/analytics         — aggregate usage stats

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};

use auth::extractor::RequireAdmin;
use shared::{
    api::{
        AdminUserListResponse, AdminUserRecord, UpdateUserGroupsRequest, UsageAnalyticsResponse,
    },
    UserGroup,
};

use crate::server::state::AppState;

// ── List users ────────────────────────────────────────────────────────────────

pub async fn list_users(
    State(state): State<AppState>,
    RequireAdmin(_admin): RequireAdmin,
) -> impl IntoResponse {
    let resp = state.cognito_client
        .list_users()
        .user_pool_id(&state.user_pool_id)
        .send()
        .await;

    match resp {
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        Ok(output) => {
            let users: Vec<AdminUserRecord> = output
                .users
                .unwrap_or_default()
                .into_iter()
                .map(|u| {
                    let id = u.username.unwrap_or_default();
                    let email = u.attributes
                        .as_deref()
                        .unwrap_or_default()
                        .iter()
                        .find(|a| a.name == "email")
                        .and_then(|a| a.value.clone())
                        .unwrap_or_default();
                    let enabled = u.enabled;
                    let created_at = u.user_create_date
                        .map(|d| d.to_string());

                    AdminUserRecord { id, email, groups: vec![], created_at, enabled }
                })
                .collect();

            Json(AdminUserListResponse { users, next_token: None }).into_response()
        }
    }
}

// ── Update groups ─────────────────────────────────────────────────────────────

pub async fn update_user_groups(
    State(state): State<AppState>,
    RequireAdmin(_admin): RequireAdmin,
    Path(user_id): Path<String>,
    Json(body): Json<UpdateUserGroupsRequest>,
) -> impl IntoResponse {
    // Add groups.
    for group in &body.add_groups {
        if group == &UserGroup::Standard { continue; }
        let result = state.cognito_client
            .admin_add_user_to_group()
            .user_pool_id(&state.user_pool_id)
            .username(&user_id)
            .group_name(group.as_cognito_name())
            .send()
            .await;
        if let Err(e) = result {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    }

    // Remove groups.
    for group in &body.remove_groups {
        if group == &UserGroup::Standard { continue; }
        let result = state.cognito_client
            .admin_remove_user_from_group()
            .user_pool_id(&state.user_pool_id)
            .username(&user_id)
            .group_name(group.as_cognito_name())
            .send()
            .await;
        if let Err(e) = result {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    }

    StatusCode::NO_CONTENT.into_response()
}

// ── Analytics ─────────────────────────────────────────────────────────────────

pub async fn analytics(
    State(_state): State<AppState>,
    RequireAdmin(_admin): RequireAdmin,
) -> impl IntoResponse {
    // Stub: aggregating across all users requires a full table scan with
    // cost-tracking data.  Return zero-filled response for now; a proper
    // implementation would scan DynamoDB or read from Cost Explorer.
    Json(UsageAnalyticsResponse {
        total_conversations: 0,
        total_input_tokens:  0,
        total_output_tokens: 0,
        estimated_cost_usd:  0.0,
        by_model:            vec![],
        top_users:           vec![],
    })
}
