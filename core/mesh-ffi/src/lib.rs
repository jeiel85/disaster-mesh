//! The only foreign-function facade exported by DisasterMesh.
//!
//! Android-facing single-owner engine facade.

#![forbid(unsafe_code)]

use std::sync::{Arc, Mutex};

use mesh_engine::{MeshRuntimeEngine, RuntimeContact};
use mesh_types::ContactId;

#[derive(Clone, Debug, Eq, PartialEq, uniffi::Record)]
pub struct ContactSummary {
    pub contact_id: Vec<u8>,
    pub display_name: String,
    pub display_id: String,
    pub safety_number: String,
    pub trust: String,
}

#[derive(Clone, Debug, Eq, PartialEq, uniffi::Record)]
pub struct DirectSend {
    pub packet_id: Vec<u8>,
    pub message_id: Vec<u8>,
    pub conversation_id: Vec<u8>,
    pub wire_bytes: Vec<u8>,
}

#[derive(Debug, uniffi::Error)]
pub enum MeshFfiError {
    InvalidArgument,
    OperationFailed,
    EngineBusy,
}

impl core::fmt::Display for MeshFfiError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("DisasterMesh operation failed")
    }
}

#[derive(uniffi::Object)]
pub struct MeshEngine {
    inner: Mutex<MeshRuntimeEngine>,
}

#[uniffi::export]
impl MeshEngine {
    #[uniffi::constructor]
    pub fn open(
        database_path: String,
        mut master_key: Vec<u8>,
        local_display_name: String,
        now_ms: u64,
    ) -> Result<Arc<Self>, MeshFfiError> {
        let key: [u8; 32] = master_key
            .as_slice()
            .try_into()
            .map_err(|_| MeshFfiError::InvalidArgument)?;
        master_key.fill(0);
        let (engine, _) = MeshRuntimeEngine::open(database_path, key, local_display_name, now_ms)
            .map_err(|_| MeshFfiError::OperationFailed)?;
        Ok(Arc::new(Self {
            inner: Mutex::new(engine),
        }))
    }

    pub fn own_contact_qr(&self, capabilities: u32) -> Result<String, MeshFfiError> {
        self.lock()?
            .own_contact_qr(capabilities)
            .map_err(|_| MeshFfiError::OperationFailed)
    }

    pub fn import_contact_qr(
        &self,
        qr: String,
        now_ms: u64,
    ) -> Result<ContactSummary, MeshFfiError> {
        let contact = self
            .lock()?
            .import_contact_qr(&qr, now_ms)
            .map_err(|_| MeshFfiError::OperationFailed)?;
        Ok(contact.into())
    }

    pub fn load_contact(&self, contact_id: Vec<u8>) -> Result<ContactSummary, MeshFfiError> {
        let contact_id = parse_contact_id(contact_id)?;
        self.lock()?
            .load_contact(contact_id)
            .map(Into::into)
            .map_err(|_| MeshFfiError::OperationFailed)
    }

    pub fn verify_contact(
        &self,
        contact_id: Vec<u8>,
        displayed_safety_number: String,
        now_ms: u64,
    ) -> Result<(), MeshFfiError> {
        let contact_id = parse_contact_id(contact_id)?;
        self.lock()?
            .verify_contact(contact_id, &displayed_safety_number, now_ms)
            .map_err(|_| MeshFfiError::OperationFailed)
    }

    pub fn send_direct_text(
        &self,
        contact_id: Vec<u8>,
        text: String,
        now_ms: u64,
        boot_id: Vec<u8>,
        elapsed_ms: u64,
    ) -> Result<DirectSend, MeshFfiError> {
        let contact_id = parse_contact_id(contact_id)?;
        let boot_id: [u8; 16] = boot_id
            .as_slice()
            .try_into()
            .map_err(|_| MeshFfiError::InvalidArgument)?;
        let sent = self
            .lock()?
            .send_direct_text(contact_id, text, now_ms, boot_id, elapsed_ms)
            .map_err(|_| MeshFfiError::OperationFailed)?;
        Ok(DirectSend {
            packet_id: sent.packet_id.as_bytes().to_vec(),
            message_id: sent.message_id.as_bytes().to_vec(),
            conversation_id: sent.conversation_id.as_bytes().to_vec(),
            wire_bytes: sent.wire_bytes,
        })
    }
}

impl MeshEngine {
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, MeshRuntimeEngine>, MeshFfiError> {
        self.inner.lock().map_err(|_| MeshFfiError::EngineBusy)
    }
}

impl From<RuntimeContact> for ContactSummary {
    fn from(value: RuntimeContact) -> Self {
        Self {
            contact_id: value.contact_id.as_bytes().to_vec(),
            display_name: value.display_name,
            display_id: value.display_id,
            safety_number: value.safety_number,
            trust: format!("{:?}", value.trust),
        }
    }
}

fn parse_contact_id(value: Vec<u8>) -> Result<ContactId, MeshFfiError> {
    let bytes: [u8; 16] = value
        .as_slice()
        .try_into()
        .map_err(|_| MeshFfiError::InvalidArgument)?;
    Ok(ContactId::from(bytes))
}

/// Returns the Rust core package version.
#[uniffi::export]
#[must_use]
pub fn version() -> String {
    mesh_engine::version()
}

uniffi::setup_scaffolding!();

#[cfg(test)]
mod tests {
    #[test]
    fn facade_version_matches_package() {
        assert_eq!(super::version(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn engine_opens_and_exports_a_signed_contact_qr() {
        let engine =
            super::MeshEngine::open(":memory:".into(), vec![7; 32], "Test".into(), 1).unwrap();
        assert!(engine.own_contact_qr(0x1f).unwrap().starts_with("DM1:"));
    }
}
