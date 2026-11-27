//! Host-selected local check integration. The runner owns admission of its
//! versioned request. Worktree owns local Git observations and their originals.
use vstd::contrib::exec_spec::ExecSpecEq;
use vstd::prelude::*;

verus! {
/// Admission at the worktree boundary precedes the runner's file identity validation.
pub(crate) fn accepts_local_request(
    schema: &str, source: &str, absolute_root: bool, has_native_artifacts: bool, prepared: bool,
) -> (accepted: bool)
    ensures accepted == (schema@ == "chrono-check-input-request/v1"@ && source@ == "local"@
        && absolute_root && !has_native_artifacts && !prepared),
{
    <&str>::exec_eq(schema, "chrono-check-input-request/v1") && <&str>::exec_eq(source, "local")
        && absolute_root && !has_native_artifacts && !prepared
}

pub(crate) fn selected_profile<'a>(
    ordinary: &'a str, initial_profile: Option<&'a str>, initial: bool, all: bool,
) -> (selected: Result<&'a str, &'static str>)
    ensures selected == match (initial, initial_profile) {
        (true, Some(profile)) => if all { Ok(profile) } else { Err("initial inventory requires a bare check") },
        _ => Ok(ordinary),
    },
{
    match (initial, initial_profile) {
        (true, Some(profile)) => if all { Ok(profile) } else { Err("initial inventory requires a bare check") },
        _ => Ok(ordinary),
    }
}
}

#[cfg(not(verus_keep_ghost))]
use crate::document::{decode, file_identity, no_symlink_parents, relative_path, sha256};
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize};
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
#[cfg(not(verus_keep_ghost))]
pub(crate) const RESPONSE: &str = "chrono-check-inputs/v1";

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, tag = "kind", rename_all = "kebab-case")]
pub(crate) enum Selection {
    All,
    Unit { unit: String },
    Collect,
}
#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, tag = "kind", rename_all = "kebab-case")]
pub(crate) enum Scope {
    Unit { unit: String },
    Collect { manifest: String },
}
#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Profile {
    pub(crate) path: String,
    pub(crate) sha256: String,
}
#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Context {
    pub(crate) path: String,
    pub(crate) raw: Vec<u8>,
    pub(crate) sha256: String,
    pub(crate) semantic_digest: String,
}
#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Original {
    pub(crate) path: String,
    pub(crate) sha256: String,
}
#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PreparedCheck {
    pub(crate) schema: String,
    pub(crate) request_sha256: String,
    pub(crate) source: String,
    pub(crate) profile: String,
    pub(crate) base: Option<String>,
    pub(crate) candidate: String,
    pub(crate) initial: bool,
    pub(crate) context: Option<Context>,
    pub(crate) scope: Option<Scope>,
    pub(crate) evidence: Value,
    pub(crate) originals: Vec<Original>,
}
// Field order matches the producer's published request serialization for
// request_sha256. Local requests require null native_artifacts and prepared.
#[cfg(not(verus_keep_ghost))]
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InputRequest {
    pub(crate) schema: String,
    pub(crate) host_root: PathBuf,
    pub(crate) source: String,
    pub(crate) host_config: String,
    pub(crate) host_config_sha256: String,
    pub(crate) effective_config: String,
    pub(crate) effective_config_sha256: String,
    pub(crate) profile: String,
    pub(crate) profile_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) initial_profile: Option<Profile>,
    pub(crate) selection: Selection,
    pub(crate) native_artifacts: Option<Value>,
    pub(crate) prepared: Option<Value>,
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn artifact_path(path: &str) -> Result<(), String> {
    relative_path(path)?;
    if path.starts_with(".chrono-harness/state/") {
        Ok(())
    } else {
        Err("check artifact must be in .chrono-harness/state/".into())
    }
}
#[cfg(not(verus_keep_ghost))]
fn query<T: serde::de::DeserializeOwned>(
    root: &Path,
    request: Value,
    timeout: u64,
    limit: usize,
    env: std::collections::BTreeMap<String, String>,
) -> Result<T, String> {
    let selected = crate::installed_tool("chrono-harness");
    let executable = crate::process::resolve_program(
        root,
        selected.to_str().ok_or("runner path not UTF-8")?,
        None,
    )?;
    let digest = file_identity(&executable)?.0;
    let command = crate::process::CommandSpec {
        program: executable.to_str().ok_or("runner path not UTF-8")?.into(),
        args: vec!["check-invocation-v1".into()],
        env,
        timeout_seconds: timeout,
        output_limit_bytes: limit,
    };
    let _custody = crate::retention::forward_scope()?;
    let result = crate::process::run_process_observed(
        root,
        &command,
        &serde_json::to_vec(&request).map_err(|e| e.to_string())?,
        &digest,
    )?;
    if let Some(error) = result.failure {
        return Err(error);
    }
    if result.exit_code != 0 {
        return Err(format!(
            "check-invocation-v1 exited {}: {}",
            result.exit_code,
            result.stderr.trim_end()
        ));
    }
    decode(&result.stdout_bytes)
}
#[cfg(not(verus_keep_ghost))]
impl InputRequest {
    pub(crate) fn validate(&self, config: &crate::Config) -> Result<(), String> {
        if !accepts_local_request(
            &self.schema,
            &self.source,
            self.host_root.is_absolute(),
            self.native_artifacts.is_some(),
            self.prepared.is_some(),
        ) {
            return Err("expected a local unprepared worktree request".into());
        }
        let (_, environment) = crate::start::environment(config)?;
        query(
            &self.host_root,
            json!({"mode":"local-preparation","request":self}),
            config.timeout_seconds,
            config.output_limit_bytes,
            environment,
        )
    }
    pub(crate) fn profile_for(&self, initial: bool) -> Result<&str, String> {
        selected_profile(
            &self.profile,
            self.initial_profile
                .as_ref()
                .map(|profile| profile.path.as_str()),
            initial,
            self.selection == Selection::All,
        )
        .map_err(String::from)
    }
}
#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Action {
    pub(crate) operation: String,
    #[serde(rename = "tool")]
    _tool: String,
    pub(crate) argv: Vec<String>,
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn participation(
    root: &Path,
    config: &Value,
    cmd: &crate::process::CommandSpec,
) -> Result<Option<Action>, String> {
    query(
        root,
        json!({"mode":"participation","config":config}),
        cmd.timeout_seconds,
        cmd.output_limit_bytes,
        cmd.env.clone(),
    )
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn original(root: &Path, path: &str) -> Result<Original, String> {
    artifact_path(path)?;
    let digest = file_identity(&no_symlink_parents(root, path)?)?.0;
    Ok(Original {
        path: path.into(),
        sha256: digest,
    })
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn retain(root: &Path, prefix: &str, value: &Value) -> Result<String, String> {
    let raw = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    let outcome = json!({"kind":"preparation-original","sha256":sha256(&raw),"length":raw.len()});
    let directory = no_symlink_parents(root, ".chrono-harness/state/preparation")?;
    fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let file = crate::retention::unique_path(&directory, prefix, ".json")?;
    let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    let relative = file
        .strip_prefix(&root)
        .map_err(|e| e.to_string())?
        .to_str()
        .ok_or("original path not UTF-8")?;
    // The input producer owns a publication read scope until it hands the
    // context to its report. Its output capability covers this publication.
    if crate::retention::publish_owned_original(&root, "check-original", relative, &raw, outcome)? {
        return Ok(relative.into());
    }
    let mut output = fs::File::create_new(&file).map_err(|e| e.to_string())?;
    output
        .write_all(&raw)
        .and_then(|()| output.sync_all())
        .map_err(|e| e.to_string())?;
    Ok(relative.into())
}
