#![warn(clippy::disallowed_types)]

mod error;
mod routes;
mod service;
mod state;

pub use error::WorkspaceError;
pub use routes::workspace_routes;
pub use service::{WORKSPACE_REFERENCE_PREFIX, WorkspaceAccess, WorkspaceAccessPort, WorkspaceService};
pub use state::WorkspaceRouterState;
