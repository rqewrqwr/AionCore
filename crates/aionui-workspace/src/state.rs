use std::sync::Arc;

use crate::WorkspaceService;

#[derive(Clone)]
pub struct WorkspaceRouterState {
    pub service: Arc<WorkspaceService>,
}
