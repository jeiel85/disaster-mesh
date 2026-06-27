//! Normative FFI contract sketch for UniFFI. Numeric persisted states come from state_codes.toml.
//! Internal crates may use richer types, but FFI compatibility must preserve field meaning.

pub type Id16 = [u8; 16];
pub type Hash32 = [u8; 32];
pub type CommandId = u64;
pub type LinkId = u64;

#[derive(Clone, Debug)]
pub struct EngineConfig {
    pub database_path: String,
    pub partial_directory: String,
    pub relay_quota_bytes: u64,
    pub local_protected_bytes: u64,
    pub protocol_major: u16,
    pub protocol_minor: u16,
    pub build_id: String,
}

#[derive(Clone)]
pub struct OpenSecrets {
    pub db_master_key: Vec<u8>, // exactly 32 bytes; zeroized after Engine::open copies it
    pub boot_id: Id16,
    pub entropy_seed_for_tests: Option<Vec<u8>>, // release build MUST reject Some
}

#[derive(Clone, Copy, Debug)]
pub enum LinkChannel { Control, Data }
#[derive(Clone, Copy, Debug)]
pub enum LinkRole { Central, Peripheral }

#[derive(Clone, Debug)]
pub enum TransportEvent {
    PeerDiscovered { peer_handle: String, beacon_id: Option<[u8;8]>, capabilities: u32, rssi: i16 },
    LinkOpened { link_id: LinkId, peer_handle: String, role: LinkRole, max_write_bytes: u32 },
    BytesReceived { link_id: LinkId, channel: LinkChannel, bytes: Vec<u8> },
    CommandCompleted { command_id: CommandId, link_id: Option<LinkId>, result_code: u32, accepted_bytes: Option<u32> },
    CommandFailed { command_id: CommandId, link_id: Option<LinkId>, category: u32 },
    LinkClosed { link_id: LinkId, reason: u32 },
    ConnectFailed { peer_handle: String, category: u32 },
}

#[derive(Clone, Debug)]
pub enum PlatformCommand {
    StartScan { command_id: CommandId, policy: u32 },
    StopScan { command_id: CommandId },
    StartAdvertising { command_id: CommandId, bytes: Vec<u8> },
    UpdateAdvertising { command_id: CommandId, bytes: Vec<u8> },
    StopAdvertising { command_id: CommandId },
    Connect { command_id: CommandId, peer_handle: String },
    Disconnect { command_id: CommandId, link_id: LinkId, reason: u32 },
    RequestMtu { command_id: CommandId, link_id: LinkId, requested: u16 },
    ConfigureNotifications { command_id: CommandId, link_id: LinkId, channel: LinkChannel, enabled: bool },
    SendBytes { command_id: CommandId, link_id: LinkId, channel: LinkChannel, bytes: Vec<u8>, with_response: bool },
    RequestConnectionPriority { command_id: CommandId, link_id: LinkId, high: bool },
    RequestLocation { command_id: CommandId, request_id: Id16, timeout_seconds: u32 },
    UpdatePersistentNotification { command_id: CommandId, model: NotificationModel },
    ShowLocalAlert { command_id: CommandId, model: LocalAlert },
    ScheduleWake { command_id: CommandId, after_ms: u64, reason: u32 },
}

#[derive(Clone, Debug)]
pub struct NotificationModel { pub state_code: u32, pub public_text_code: u32 }
#[derive(Clone, Debug)]
pub struct LocalAlert { pub public_code: u32, pub related_id: Option<Id16> }

#[derive(Clone, Debug)]
pub struct CoreError {
    pub public_code: u32,
    pub internal_code: u32,
    pub retryable: bool,
    pub related_id: Option<Id16>,
}

pub struct MeshEngine { inner: std::sync::Mutex<mesh_engine::Engine> }

impl MeshEngine {
    pub fn open(config: EngineConfig, secrets: OpenSecrets) -> Result<Self, CoreError> {
        if secrets.db_master_key.len() != 32 { return Err(CoreError { public_code: 1, internal_code: 1001, retryable: false, related_id: None }); }
        #[cfg(not(test))]
        if secrets.entropy_seed_for_tests.is_some() { return Err(CoreError { public_code: 1, internal_code: 1002, retryable: false, related_id: None }); }
        let inner = mesh_engine::Engine::open(config.into(), secrets.into()).map_err(CoreError::from)?;
        Ok(Self { inner: std::sync::Mutex::new(inner) })
    }

    pub fn handle_transport_event(&self, event: TransportEvent) -> Result<Vec<PlatformCommand>, CoreError> {
        let mut engine = self.inner.lock().map_err(|_| CoreError { public_code: 1, internal_code: 1003, retryable: false, related_id: None })?;
        engine.handle_transport_event(event.into()).map_err(Into::into)
    }
}
