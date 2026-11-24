use crate::process::CommandSpec;
use serde_json::{Value, json};
use std::collections::BTreeMap;

/// Query the configured workflow command and distinguish stale observations from failed checks.
pub fn check(
    context: &Value,
    workflow: &Value,
    program: &str,
    timeout_seconds: u64,
    output_limit_bytes: usize,
    environment: &BTreeMap<String, String>,
    run: impl FnOnce(&CommandSpec, &[u8]) -> Result<(i32, Vec<u8>, Vec<u8>), String>,
) -> Result<bool, String> {
    let input = json!({
        "context": {
            "observed_at": &context["observed_at"],
            "branch_started_at": &context["branch_started_at"],
        },
        "workflow": {"staleness": {"max_age_hours": &workflow["staleness"]["max_age_hours"]}},
    });
    let command = CommandSpec {
        program: program.into(),
        args: vec!["check-observation-age".into()],
        env: environment.clone(),
        timeout_seconds,
        output_limit_bytes,
    };
    let (code, stdout, stderr) = run(
        &command,
        &serde_json::to_vec(&input).map_err(|e| e.to_string())?,
    )?;
    let diagnostic = String::from_utf8(stderr).map_err(|e| e.to_string())?;
    let message = diagnostic.strip_suffix('\n').unwrap_or(&diagnostic);
    match code {
        0 if stdout.is_empty() && message.is_empty() => Ok(true),
        1 if stdout.is_empty() && message.starts_with("E_BRANCH_STALE:") => Ok(false),
        2 if stdout.is_empty() && !message.is_empty() => Err(message.to_owned()),
        _ => Err(format!(
            "workflow age check returned {code}: {message}; stdout={stdout:?}"
        )),
    }
}
