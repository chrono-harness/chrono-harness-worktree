//! The owner of the worktree maintenance socket accepts bounded, nonblocking wake-up hints.
use vstd::prelude::*;

verus! {
pub(crate) fn valid_hint_length(length: usize) -> (valid: bool)
    ensures valid == (0 < length && length <= 64),
{
    length > 0 && length <= 64
}

/// Only adopted or failed wake requests have a receipt. Keep the original failure.
pub(crate) fn wake_receipt<E>(wake: Result<bool, E>) -> (receipt: Option<(&'static str, Option<E>)>)
    ensures match wake {
        Ok(false) => receipt.is_none(),
        Ok(true) => receipt matches Some((status, None)) && status@ == "requested"@,
        Err(error) => receipt matches Some((status, Some(original))) &&
            status@ == "wake-failed"@ && original == error,
    },
{
    match wake {
        Ok(false) => None,
        Ok(true) => Some(("requested", None)),
        Err(error) => Some(("wake-failed", Some(error))),
    }
}
}

#[cfg(not(verus_keep_ghost))]
use crate::document::{decode, no_symlink_parents, relative_path, sha256};
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize};
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::{
    cell::Cell,
    fs,
    io::{self, ErrorKind, Write},
    os::unix::net::UnixDatagram,
    path::{Component, Path, PathBuf},
};

#[cfg(not(verus_keep_ghost))]
pub(crate) const POLICY_PATH: &str = ".chrono-harness/maintenance.json";
#[cfg(not(verus_keep_ghost))]
pub(crate) const CONTEXT_ENV: &str = "CHRONO_MAINTENANCE_CONTEXT";

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RequestContext {
    source_root: PathBuf,
    owner_root: PathBuf,
}
#[cfg(not(verus_keep_ghost))]
impl RequestContext {
    fn roots(&self) -> [&Path; 2] {
        [&self.source_root, &self.owner_root]
    }
    fn check_shape(&self) -> Result<(), String> {
        if self.roots().iter().any(|root| {
            !root.is_absolute()
                || root
                    .components()
                    .any(|part| matches!(part, Component::CurDir | Component::ParentDir))
        }) {
            Err("maintenance context requires physical absolute roots".into())
        } else {
            Ok(())
        }
    }
    fn check(&self) -> Result<(), String> {
        self.check_shape()?;
        for path in self.roots() {
            if fs::canonicalize(path).map_err(|e| e.to_string())? != path {
                return Err("maintenance context requires physical absolute roots".into());
            }
        }
        Ok(())
    }
    pub(crate) fn new(source_root: &Path, owner_root: &Path) -> Result<Self, String> {
        let context = Self {
            source_root: source_root.into(),
            owner_root: owner_root.into(),
        };
        context.check()?;
        Ok(context)
    }
}

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub schema: String,
    pub state_directory: String,
    pub socket: String,
}
#[cfg(not(verus_keep_ghost))]
impl Policy {
    pub(crate) fn validate(&self) -> Result<(), String> {
        relative_path(self.state_directory.trim_end_matches('/'))?;
        relative_path(&self.socket)?;
        if self.schema != "chrono-unified-maintenance/v2"
            || !self.state_directory.starts_with(".chrono-harness/state/")
            || !self.state_directory.ends_with('/')
            || !self.socket.starts_with(".chrono-harness/state/")
            || self.socket.ends_with('/')
        {
            return Err("invalid unified maintenance policy".into());
        }
        Ok(())
    }
    pub(crate) fn socket_path(&self, root: &Path) -> Result<PathBuf, String> {
        self.validate()?;
        no_symlink_parents(root, &self.socket)
    }
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn adopted(root: &Path) -> Result<Option<(Policy, Vec<u8>)>, String> {
    let bytes = match fs::read(no_symlink_parents(root, POLICY_PATH)?) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    let policy: Policy = decode(&bytes)?;
    policy.validate()?;
    Ok(Some((policy, bytes)))
}

#[cfg(not(verus_keep_ghost))]
thread_local! {static DEPTH:Cell<usize>=const{Cell::new(0)};}
#[cfg(not(verus_keep_ghost))]
pub(crate) struct RoundGuard;
#[cfg(not(verus_keep_ghost))]
impl RoundGuard {
    pub(crate) fn active() -> bool {
        DEPTH.with(|depth| depth.get() > 0)
    }
    pub(crate) fn enter() -> Self {
        DEPTH.with(|depth| depth.set(depth.get() + 1));
        Self
    }
}
#[cfg(not(verus_keep_ghost))]
impl Drop for RoundGuard {
    fn drop(&mut self) {
        DEPTH.with(|depth| depth.set(depth.get() - 1));
    }
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn send(path: &Path, hint: &[u8]) -> Result<(), String> {
    if !valid_hint_length(hint.len()) {
        return Err("invalid maintenance activation hint length".into());
    }
    let socket = UnixDatagram::unbound().map_err(|e| e.to_string())?;
    socket.set_nonblocking(true).map_err(|e| e.to_string())?;
    let written = socket.send_to(hint, path).map_err(|e| e.to_string())?;
    if written != hint.len() {
        return Err("incomplete maintenance activation hint".into());
    }
    Ok(())
}

#[cfg(not(verus_keep_ghost))]
pub fn announce_adopted(root: &Path, reason: &str) {
    if RoundGuard::active() {
        return;
    }
    let mut receipt = json!({"schema":"chrono-maintenance-request/v1","reason":reason,"accepted":false,
        "round_completion":"not-established","disposal":"not-attempted","responsibility":"existing-owner-state"});
    let result = (|| -> Result<bool, String> {
        let context: Option<RequestContext> = std::env::var(CONTEXT_ENV)
            .ok()
            .map(|raw| decode(raw.as_bytes()))
            .transpose()?;
        if let Some(context) = &context {
            context.check_shape()?;
        }
        let route = context.as_ref().filter(|r| r.source_root == root);
        if let Some(route) = route {
            route.check()?;
        }
        let root = route.map_or(root, |r| r.owner_root.as_path());
        receipt["activation_root"] = json!(root.to_str());
        let Some((policy, bytes)) = adopted(root)? else {
            return Ok(false);
        };
        receipt["policy_sha256"] = json!(sha256(&bytes));
        let socket = policy.socket_path(root)?;
        receipt["socket"] = json!(socket.to_str());
        send(&socket, reason.as_bytes())?;
        Ok(true)
    })();
    let Some((status, error)) = wake_receipt(result) else {
        return;
    };
    receipt["status"] = json!(status);
    receipt["accepted"] = Value::Bool(error.is_none());
    if let Some(error) = error {
        receipt["error"] = json!(error);
    }
    // The owner has already handed off the work, even if the notice has no reader.
    let _ = writeln!(io::stderr().lock(), "W_MAINTENANCE_ACTIVATION: {receipt}");
}
