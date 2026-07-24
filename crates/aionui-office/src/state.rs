use aionui_db::IConversationRepository;
use dashmap::DashMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::conversion::ConversionService;
use crate::proxy::ProxyService;
use crate::snapshot::SnapshotService;
use crate::watch_manager::OfficecliWatchManager;

#[derive(Clone)]
pub struct OfficeRouterState {
    pub watch_manager: Arc<OfficecliWatchManager>,
    pub snapshot_service: Arc<SnapshotService>,
    pub conversion_service: Arc<ConversionService>,
    pub proxy_service: Arc<ProxyService>,
    pub allowed_roots: Vec<PathBuf>,
    pub conversation_repo: Option<Arc<dyn IConversationRepository>>,
    pub local_mode: bool,
    pub preview_port_owners: Arc<DashMap<u16, String>>,
    pub preview_path_owners: Arc<DashMap<String, String>>,
}
