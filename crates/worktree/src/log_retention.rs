use crate::process::CommandSpec;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LogRetention {
    pub removed: usize,
    pub legacy: usize,
    pub busy: bool,
    pub errors: Vec<String>,
}

/// Invoke the installed log owner, preserving partial results and original diagnostics.
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
    if !matches!(exit, 0 | 1) || !stderr.is_empty() {
        return Err(format!(
            "log owner exited {exit}: {}",
            String::from_utf8_lossy(&stderr).trim_end()
        ));
    }
    let result: Option<LogRetention> = crate::document::decode(&stdout)?;
    let partial = result.as_ref().is_some_and(|r| !r.errors.is_empty());
    if (exit == 1) != partial {
        return Err(format!(
            "log owner exit {exit} contradicts its expiry report"
        ));
    }
    Ok(result)
}
