//! Integration tests verifying that --work-dir is used for conversation workspace creation.

use aionui_api_types::CreateConversationRequest;
use aionui_app::{AppConfig, AppServices, build_conversation_state};
use aionui_common::AgentType;
use aionui_db::{IWorkspaceRepository, SqliteWorkspaceRepository};

#[tokio::test]
async fn conversation_workspace_uses_work_dir() {
    let data_dir = tempfile::TempDir::new().unwrap();
    let work_dir = tempfile::TempDir::new().unwrap();

    let db = aionui_db::init_database_memory().await.unwrap();
    let config = AppConfig {
        data_dir: data_dir.path().to_path_buf(),
        work_dir: work_dir.path().to_path_buf(),
        local: true,
        ..Default::default()
    };
    let services = AppServices::from_config(db, &config).await.unwrap();
    let state = build_conversation_state(&services, None, None);

    let request = CreateConversationRequest {
        r#type: Some(AgentType::Acp),
        name: Some("test".to_string()),
        model: None,
        assistant: None,
        source: None,
        channel_chat_id: None,
        extra: serde_json::json!({}),
    };
    let response = state.service.create("system_default_user", request).await.unwrap();

    let workspace = response.extra.get("workspace").and_then(|v| v.as_str()).unwrap();
    assert!(
        workspace.starts_with(work_dir.path().to_str().unwrap()),
        "workspace should be under work_dir, got: {workspace}"
    );
    assert!(
        !workspace.starts_with(data_dir.path().to_str().unwrap()),
        "workspace should NOT be under data_dir, got: {workspace}"
    );
}

#[tokio::test]
async fn user_specified_workspace_is_not_overridden() {
    let data_dir = tempfile::TempDir::new().unwrap();
    let work_dir = tempfile::TempDir::new().unwrap();
    let custom_workspace = tempfile::TempDir::new().unwrap();

    let db = aionui_db::init_database_memory().await.unwrap();
    let config = AppConfig {
        data_dir: data_dir.path().to_path_buf(),
        work_dir: work_dir.path().to_path_buf(),
        local: true,
        ..Default::default()
    };
    let services = AppServices::from_config(db, &config).await.unwrap();
    let state = build_conversation_state(&services, None, None);

    let request = CreateConversationRequest {
        r#type: Some(AgentType::Acp),
        name: Some("test".to_string()),
        model: None,
        assistant: None,
        source: None,
        channel_chat_id: None,
        extra: serde_json::json!({
            "workspace": custom_workspace.path().to_str().unwrap()
        }),
    };
    let response = state.service.create("system_default_user", request).await.unwrap();

    let workspace = response.extra.get("workspace").and_then(|v| v.as_str()).unwrap();
    assert!(
        workspace.starts_with(custom_workspace.path().to_str().unwrap()),
        "workspace should use user-specified path, got: {workspace}"
    );
}

#[tokio::test]
async fn workspace_defaults_to_data_dir_when_work_dir_equals_data_dir() {
    let data_dir = tempfile::TempDir::new().unwrap();

    let db = aionui_db::init_database_memory().await.unwrap();
    let config = AppConfig {
        data_dir: data_dir.path().to_path_buf(),
        work_dir: data_dir.path().to_path_buf(),
        local: true,
        ..Default::default()
    };
    let services = AppServices::from_config(db, &config).await.unwrap();
    let state = build_conversation_state(&services, None, None);

    let request = CreateConversationRequest {
        r#type: Some(AgentType::Acp),
        name: Some("test".to_string()),
        model: None,
        assistant: None,
        source: None,
        channel_chat_id: None,
        extra: serde_json::json!({}),
    };
    let response = state.service.create("system_default_user", request).await.unwrap();

    let workspace = response.extra.get("workspace").and_then(|v| v.as_str()).unwrap();
    assert!(
        workspace.starts_with(data_dir.path().to_str().unwrap()),
        "workspace should be under data_dir when work_dir == data_dir, got: {workspace}"
    );
}

#[tokio::test]
async fn multi_user_conversation_gets_owned_managed_workspace() {
    let data_dir = tempfile::TempDir::new().unwrap();
    let work_dir = tempfile::TempDir::new().unwrap();
    let db = aionui_db::init_database_memory().await.unwrap();
    let config = AppConfig {
        data_dir: data_dir.path().to_path_buf(),
        work_dir: work_dir.path().to_path_buf(),
        local: false,
        ..Default::default()
    };
    let services = AppServices::from_config(db, &config).await.unwrap();
    let state = build_conversation_state(&services, None, None);
    let owner = services
        .user_repo
        .create_user("workspace-owner", "test-hash")
        .await
        .unwrap();
    let request = CreateConversationRequest {
        r#type: Some(AgentType::Acp),
        name: Some("private project".to_string()),
        model: None,
        assistant: None,
        source: None,
        channel_chat_id: None,
        extra: serde_json::json!({}),
    };

    let response = state.service.create(&owner.id, request).await.unwrap();
    let workspace_path = response.extra["workspace"].as_str().unwrap();
    let workspace_id = response.extra["workspace_id"].as_str().unwrap();
    let repository = SqliteWorkspaceRepository::new(services.database.pool().clone());

    assert!(workspace_path.starts_with(work_dir.path().join("workspaces").to_str().unwrap()));
    assert!(repository.get_owned(&owner.id, workspace_id).await.unwrap().is_some());
    assert!(repository.get_owned("user-b", workspace_id).await.unwrap().is_none());

    let bound = repository
        .get_for_conversation(&owner.id, &response.id)
        .await
        .unwrap()
        .expect("conversation workspace binding");
    assert_eq!(bound.id, workspace_id);
}
