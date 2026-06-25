//! Contract sketch. Adapt imports/types to the selected UniFFI style.
//! Business logic must remain in internal crates.

pub struct MeshEngine {
    inner: std::sync::Mutex<mesh_engine::Engine>,
}

impl MeshEngine {
    pub fn open(config: EngineConfig, secrets: OpenSecrets) -> Result<Self, CoreError> {
        let inner = mesh_engine::Engine::open(config.into(), secrets.into())?;
        Ok(Self { inner: std::sync::Mutex::new(inner) })
    }

    pub fn handle_transport_event(
        &self,
        event: TransportEvent,
    ) -> Result<Vec<PlatformCommand>, CoreError> {
        let mut engine = self.inner.lock().map_err(|_| CoreError::poisoned())?;
        engine.handle_transport_event(event.into()).map_err(Into::into)
    }

    pub fn handle_system_event(
        &self,
        event: SystemEvent,
    ) -> Result<Vec<PlatformCommand>, CoreError> {
        let mut engine = self.inner.lock().map_err(|_| CoreError::poisoned())?;
        engine.handle_system_event(event.into()).map_err(Into::into)
    }
}
