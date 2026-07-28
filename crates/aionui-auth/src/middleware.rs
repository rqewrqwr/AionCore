#![allow(clippy::disallowed_types)]

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;

use aionui_common::ApiError;
use aionui_db::IUserRepository;

use crate::extract::extract_token_from_headers;
use crate::{AGENT_SKILL_CONFIG_SCOPE, JwtService, TokenPayload};

pub const PRIVATE_ASSET_GATEWAY_SECRET_ENV: &str = "AIONUI_PRIVATE_GATEWAY_SECRET";
pub const PRIVATE_ASSET_GATEWAY_SECRET_HEADER: &str = "x-aionui-private-gateway-secret";

/// Authenticated user injected into request extensions by the auth middleware.
///
/// Route handlers extract this from `request.extensions()` to identify
/// the current user.
#[derive(Debug, Clone)]
pub struct CurrentUser {
    /// User ID from the database.
    pub id: String,
    /// Username.
    pub username: String,
}

/// Shared state for the authentication middleware.
#[derive(Clone)]
pub struct AuthState {
    pub jwt_service: Arc<JwtService>,
    pub user_repo: Arc<dyn IUserRepository>,
    /// When `true`, skip JWT verification and inject a fixed default user.
    pub local: bool,
    /// Shared only by AionCore and the loopback private-asset gateway.
    /// Agent subprocess environments must never inherit this value.
    pub private_asset_gateway_secret: Option<String>,
}

/// Authentication middleware that verifies JWT tokens and injects `CurrentUser`.
///
/// Flow:
/// 1. Extract bearer token from `Authorization` header or `aionui-session` cookie
/// 2. Verify JWT signature, expiration, and blacklist
/// 3. Look up user in the database to ensure they still exist
/// 4. Insert [`CurrentUser`] into request extensions
///
/// Returns HTTP 401 for authentication failures.
///
/// Use with `axum::middleware::from_fn_with_state`.
pub async fn auth_middleware(
    State(state): State<AuthState>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // In local mode, skip JWT verification and inject a fixed default user.
    if state.local {
        request.extensions_mut().insert(CurrentUser {
            id: "system_default_user".to_string(),
            username: "system_default_user".to_string(),
        });
        return Ok(next.run(request).await);
    }

    let token = extract_token_from_headers(request.headers())
        .ok_or_else(|| ApiError::Unauthorized("Authentication required".into()))?;

    let payload = state.jwt_service.verify(&token).map_err(|e| {
        tracing::debug!("Token verification failed: {e}");
        ApiError::Unauthorized("Invalid or expired token".into())
    })?;

    authorize_scoped_runtime_token(&payload, &request, state.private_asset_gateway_secret.as_deref())?;

    let user = state
        .user_repo
        .find_by_id(&payload.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "auth middleware user lookup failed");
            ApiError::Internal("Authentication service unavailable".into())
        })?
        .ok_or_else(|| ApiError::Unauthorized("Invalid authentication subject".into()))?;

    request.extensions_mut().insert(CurrentUser {
        id: user.id,
        username: user.username,
    });

    Ok(next.run(request).await)
}

fn authorize_scoped_runtime_token(
    payload: &TokenPayload,
    request: &Request,
    private_asset_gateway_secret: Option<&str>,
) -> Result<(), ApiError> {
    if payload.scope.as_deref() != Some(AGENT_SKILL_CONFIG_SCOPE) {
        return Ok(());
    }
    let method = request.method().as_str();
    let path = request.uri().path();
    if method == "GET" && path == "/api/auth/user" {
        return Ok(());
    }
    let expected_conversation = payload.conversation_id.as_deref().unwrap_or_default();
    let header = |name: &str| request.headers().get(name).and_then(|value| value.to_str().ok());
    if header("x-aionui-user-id") != Some(payload.user_id.as_str())
        || header("x-aionui-conversation-id") != Some(expected_conversation)
    {
        return Err(ApiError::Forbidden("Agent runtime token context mismatch".into()));
    }
    let conversation_path = format!("/api/conversations/{expected_conversation}");
    let private_asset_route = [
        "/api/skills",
        "/api/assistants",
        "/api/providers",
        "/api/mcp",
        "/api/agents",
    ]
    .into_iter()
    .any(|root| route_is_or_has_child(path, root));
    let conversation_cron_route = is_conversation_cron_runtime_route(method, path);
    let allowed = (method == "GET" && path == conversation_path)
        || conversation_cron_route
        || (private_asset_route && private_asset_gateway_is_trusted(request, private_asset_gateway_secret));
    if !allowed {
        return Err(ApiError::Forbidden(
            "Agent runtime token is not authorized for this route".into(),
        ));
    }
    Ok(())
}

pub(crate) fn private_asset_gateway_is_trusted(request: &Request, expected: Option<&str>) -> bool {
    let Some(expected) = expected.filter(|value| !value.is_empty()) else {
        return false;
    };
    let Some(actual) = request
        .headers()
        .get(PRIVATE_ASSET_GATEWAY_SECRET_HEADER)
        .and_then(|value| value.to_str().ok())
    else {
        return false;
    };
    constant_time_eq(expected.as_bytes(), actual.as_bytes())
}

fn constant_time_eq(expected: &[u8], actual: &[u8]) -> bool {
    if expected.len() != actual.len() {
        return false;
    }
    expected
        .iter()
        .zip(actual)
        .fold(0_u8, |difference, (left, right)| difference | (left ^ right))
        == 0
}

fn route_is_or_has_child(path: &str, root: &str) -> bool {
    path == root || path.strip_prefix(root).is_some_and(|suffix| suffix.starts_with('/'))
}

fn is_conversation_cron_runtime_route(method: &str, path: &str) -> bool {
    match (method, path) {
        ("GET", "/api/internal/conversation-cron/list") | ("POST", "/api/internal/conversation-cron/create") => true,
        ("PUT", path) => path
            .strip_prefix("/api/internal/conversation-cron/jobs/")
            .is_some_and(|job_id| !job_id.is_empty() && !job_id.contains('/')),
        _ => false,
    }
}

/// Local-mode authentication middleware that skips JWT verification.
///
/// Injects a fixed `CurrentUser` with id and username `system_default_user`.
/// Used when the server runs as an embedded subprocess inside Electron.
pub async fn local_auth_middleware(mut request: Request, next: Next) -> Response {
    request.extensions_mut().insert(CurrentUser {
        id: "system_default_user".to_string(),
        username: "system_default_user".to_string(),
    });
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
    use tower::ServiceExt;

    async fn echo_user(request: Request<Body>) -> String {
        let user = request.extensions().get::<CurrentUser>().unwrap();
        format!("{}:{}", user.id, user.username)
    }

    #[tokio::test]
    async fn test_local_auth_middleware_injects_default_user() {
        let app = Router::new()
            .route("/test", get(echo_user))
            .route_layer(axum::middleware::from_fn(local_auth_middleware));

        let response = app
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(
            std::str::from_utf8(&body).unwrap(),
            "system_default_user:system_default_user"
        );
    }

    #[test]
    fn scoped_runtime_token_enforces_route_and_context() {
        let payload = TokenPayload {
            user_id: "user-1".into(),
            username: String::new(),
            iat: 1,
            exp: u64::MAX,
            iss: "aionui".into(),
            aud: "aionui-webui".into(),
            scope: Some(AGENT_SKILL_CONFIG_SCOPE.into()),
            conversation_id: Some("conv-1".into()),
        };
        let wrong_route = Request::builder()
            .uri("/api/providers")
            .header("x-aionui-user-id", "user-1")
            .header("x-aionui-conversation-id", "conv-1")
            .body(Body::empty())
            .unwrap();
        assert!(authorize_scoped_runtime_token(&payload, &wrong_route, Some("gateway-secret")).is_err());
        let valid = Request::builder()
            .uri("/api/providers")
            .header("x-aionui-user-id", "user-1")
            .header("x-aionui-conversation-id", "conv-1")
            .header(PRIVATE_ASSET_GATEWAY_SECRET_HEADER, "gateway-secret")
            .body(Body::empty())
            .unwrap();
        assert!(authorize_scoped_runtime_token(&payload, &valid, Some("gateway-secret")).is_ok());
        let wrong_context = Request::builder()
            .uri("/api/providers")
            .header("x-aionui-user-id", "user-2")
            .header("x-aionui-conversation-id", "conv-1")
            .header(PRIVATE_ASSET_GATEWAY_SECRET_HEADER, "gateway-secret")
            .body(Body::empty())
            .unwrap();
        assert!(authorize_scoped_runtime_token(&payload, &wrong_context, Some("gateway-secret")).is_err());
    }

    #[test]
    fn scoped_runtime_token_rejects_spoofed_gateway_secret() {
        let payload = TokenPayload {
            user_id: "user-1".into(),
            username: String::new(),
            iat: 1,
            exp: u64::MAX,
            iss: "aionui".into(),
            aud: "aionui-webui".into(),
            scope: Some(AGENT_SKILL_CONFIG_SCOPE.into()),
            conversation_id: Some("conv-1".into()),
        };
        let request = Request::builder()
            .uri("/api/mcp/servers")
            .header("x-aionui-user-id", "user-1")
            .header("x-aionui-conversation-id", "conv-1")
            .header(PRIVATE_ASSET_GATEWAY_SECRET_HEADER, "wrong-secret")
            .body(Body::empty())
            .unwrap();

        assert!(authorize_scoped_runtime_token(&payload, &request, Some("gateway-secret")).is_err());
    }

    #[test]
    fn scoped_runtime_token_allows_only_conversation_cron_current_routes_for_its_context() {
        let payload = TokenPayload {
            user_id: "user-1".into(),
            username: String::new(),
            iat: 1,
            exp: u64::MAX,
            iss: "aionui".into(),
            aud: "aionui-webui".into(),
            scope: Some(AGENT_SKILL_CONFIG_SCOPE.into()),
            conversation_id: Some("conv-1".into()),
        };
        let request = |method: &str, path: &str, user_id: &str, conversation_id: &str| {
            Request::builder()
                .method(method)
                .uri(path)
                .header("x-aionui-user-id", user_id)
                .header("x-aionui-conversation-id", conversation_id)
                .body(Body::empty())
                .unwrap()
        };

        for (method, path) in [
            ("GET", "/api/internal/conversation-cron/list"),
            ("POST", "/api/internal/conversation-cron/create"),
            ("PUT", "/api/internal/conversation-cron/jobs/cron-1"),
        ] {
            assert!(
                authorize_scoped_runtime_token(
                    &payload,
                    &request(method, path, "user-1", "conv-1"),
                    Some("gateway-secret")
                )
                .is_ok(),
                "{method} {path} should be allowed"
            );
        }

        for (method, path) in [
            ("POST", "/api/internal/conversation-cron/list"),
            ("GET", "/api/internal/conversation-cron/create"),
            ("GET", "/api/internal/conversation-cron/jobs/cron-1"),
            ("PUT", "/api/internal/conversation-cron/jobs/"),
            ("PUT", "/api/internal/conversation-cron/jobs/cron-1/run"),
            ("GET", "/api/cron/jobs"),
        ] {
            assert!(
                authorize_scoped_runtime_token(
                    &payload,
                    &request(method, path, "user-1", "conv-1"),
                    Some("gateway-secret")
                )
                .is_err(),
                "{method} {path} should be rejected"
            );
        }

        assert!(
            authorize_scoped_runtime_token(
                &payload,
                &request("GET", "/api/internal/conversation-cron/list", "other-user", "conv-1"),
                Some("gateway-secret")
            )
            .is_err()
        );
        assert!(
            authorize_scoped_runtime_token(
                &payload,
                &request(
                    "GET",
                    "/api/internal/conversation-cron/list",
                    "user-1",
                    "other-conversation"
                ),
                Some("gateway-secret")
            )
            .is_err()
        );
    }
}
