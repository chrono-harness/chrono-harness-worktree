//! Interpret the diagnostics owner's expiry result for worktree maintenance.
#![allow(unexpected_cfgs)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
use super::accept_log_report;
#[cfg(not(verus_keep_ghost))]
use crate::verified_lifecycle::accept_log_report;

#[cfg(not(verus_keep_ghost))]
use crate::process::CommandSpec;
#[cfg(not(verus_keep_ghost))]
use std::collections::BTreeMap;
#[cfg(not(verus_keep_ghost))]
use std::path::Path;

verus! {
#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Deserialize, serde::Serialize))]
#[cfg_attr(not(verus_keep_ghost), serde(deny_unknown_fields))]
pub struct LogRetention {
    pub removed: usize,
    pub legacy: usize,
    pub busy: bool,
    pub errors: Vec<String>,
}

/// A partial expiry must retain the original counters, busy indication and ordered errors.
pub fn accept_owner_report(exit: i32, report: Option<LogRetention>)
    -> (result: Result<Option<LogRetention>, ()>)
    ensures
        result.is_ok() == match report {
            None => exit == 0,
            Some(logs) => if logs.errors.len() == 0 { exit == 0 } else { exit == 1 },
        },
        result.is_ok() ==> result == Ok::<_, ()>(report),
{
    let has_errors = match &report {
        None => false,
        Some(logs) => !logs.errors.is_empty(),
    };
    accept_log_report(exit, report, has_errors)
}
}

/// Invoke the installed log owner, preserving partial results and original diagnostics.
#[cfg(not(verus_keep_ghost))]
pub fn expire(
    root: &Path,
    executable: &str,
    timeout_seconds: u64,
    output_limit_bytes: usize,
    environment: &BTreeMap<String, String>,
    run: impl FnOnce(&CommandSpec) -> Result<(i32, Vec<u8>, Vec<u8>), String>,
) -> Result<Option<LogRetention>, String> {
    let command = CommandSpec {
        program: executable.into(),
        args: vec![
            "expire-logs-v1".into(),
            root.to_str().ok_or("log root must be UTF-8")?.into(),
        ],
        env: environment.clone(),
        timeout_seconds,
        output_limit_bytes,
    };
    let (exit, stdout, stderr) = run(&command)?;
    if !crate::verified_lifecycle::log_owner_can_report(exit, stderr.is_empty()) {
        return Err(format!(
            "log owner exited {exit}: {}",
            String::from_utf8_lossy(&stderr).trim_end()
        ));
    }
    let result: Option<LogRetention> = crate::document::decode(&stdout)?;
    accept_owner_report(exit, result)
        .map_err(|()| format!("log owner exit {exit} contradicts its expiry report"))
}
