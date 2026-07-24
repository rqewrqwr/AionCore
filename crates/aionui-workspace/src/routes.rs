use axum::Router;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Extension, Json, State};
use axum::routing::get;

use aionui_api_types::{ApiResponse, CreateWorkspaceRequest, WorkspaceListResponse, WorkspaceResponse};
use aionui_auth::CurrentUser;
use aionui_common::ApiError;

use crate::state::WorkspaceRouterState;

pub fn workspace_routes(state: WorkspaceRouterState) -> Router {
    Router::new()
        .route("/api/workspaces", get(list_workspaces).post(create_workspace))
        .with_state(state)
}

async fn list_workspaces(
    State(state): State<WorkspaceRouterState>,
    Extension(user): Extension<CurrentUser>,
) -> Result<Json<ApiResponse<WorkspaceListResponse>>, ApiError> {
    let workspaces = state.service.list_personal(&user.id).await?;
    Ok(Json(ApiResponse::ok(workspaces)))
}

async fn create_workspace(
    State(state): State<WorkspaceRouterState>,
    Extension(user): Extension<CurrentUser>,
    body: Result<Json<CreateWorkspaceRequest>, JsonRejection>,
) -> Result<Json<ApiResponse<WorkspaceResponse>>, ApiError> {
    let Json(request) = body.map_err(ApiError::from)?;
    if request.name.trim().is_empty() {
        return Err(ApiError::BadRequest("Workspace name is required".to_owned()));
    }
    let workspace = state.service.create_personal(&user.id, &request.name).await?;
    Ok(Json(ApiResponse::ok(workspace)))
}
