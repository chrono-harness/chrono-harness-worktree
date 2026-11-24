//! Exclude overlapping final-original publications while removing outputs.
use crate::retention_session::Session;
use serde_json::json;
use std::{cell::RefCell, fs, os::fd::AsFd, path::Path};

pub(crate) struct Guard {
    session: RefCell<Session>,
    // The subprocess and any deliberately handed-off child share the original open descriptions.
    _capabilities: chrono_harness::process_fds::Scope,
}

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
        if fds.is_empty() {
            return Err("missing removal descriptors".into());
        }
        let borrowed = fds.iter().map(AsFd::as_fd).collect::<Vec<_>>();
        let capabilities = chrono_harness::process_fds::Scope::new(&borrowed)?;
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
