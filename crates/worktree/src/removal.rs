//! Exclude overlapping final-original publications while removing outputs.
#![allow(unexpected_cfgs)]
use vstd::prelude::*;

verus! {
/// An opened removal scope must come with an actual lease.
pub(crate) fn require_descriptors(count: usize) -> (result: Result<(), &'static str>)
    ensures
        result.is_ok() == (count > 0),
        count == 0 ==> result == Err::<(), &str>("missing removal descriptors"),
{
    if count == 0 { Err("missing removal descriptors") } else { Ok(()) }
}
}

#[cfg(not(verus_keep_ghost))]
use crate::retention_session::Session;
#[cfg(not(verus_keep_ghost))]
use serde_json::json;
#[cfg(not(verus_keep_ghost))]
use std::{cell::RefCell, fs, os::fd::AsFd, path::Path};

#[cfg(not(verus_keep_ghost))]
pub(crate) struct Guard {
    session: RefCell<Session>,
    // The subprocess and any deliberately handed-off child share the original open descriptions.
    _capabilities: crate::process_fds::Scope,
}

#[cfg(not(verus_keep_ghost))]
impl Guard {
    pub(crate) fn one(root: &Path, path: &str) -> Result<Option<Self>, String> {
        Self::open(root, &[path.to_owned()])
    }

    pub(crate) fn open(root: &Path, paths: &[String]) -> Result<Option<Self>, String> {
        for path in paths {
            crate::document::relative_path(path)?;
        }
        match fs::symlink_metadata(root.join(".chrono-harness/retention.json")) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.to_string()),
            Ok(_) => {}
        }
        let mut session = Session::open(root)?;
        if !session.call::<bool>(json!({"operation":"open-removal","paths":paths}))? {
            return Ok(None);
        }
        let fds = session.descriptors()?;
        require_descriptors(fds.len())?;
        let borrowed = fds.iter().map(AsFd::as_fd).collect::<Vec<_>>();
        let capabilities = crate::process_fds::Scope::new(&borrowed)?;
        Ok(Some(Self {
            session: RefCell::new(session),
            _capabilities: capabilities,
        }))
    }

    pub(crate) fn stable(&self) -> Result<(), String> {
        self.session
            .borrow_mut()
            .call(json!({"operation":"check-removal"}))
    }
}
