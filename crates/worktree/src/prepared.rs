//! Host-selected local check integration. The runner owns admission of its
//! versioned request. Worktree owns local Git observations and their originals.
use crate::document::{decode, file_identity, no_symlink_parents, relative_path, sha256};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
pub(crate) const RESPONSE: &str = "chrono-check-inputs/v1";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, tag = "kind", rename_all = "kebab-case")]
pub(crate) enum Selection {
    All,
    Unit { unit: String },
    Collect,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, tag = "kind", rename_all = "kebab-case")]
pub(crate) enum Scope {
    Unit { unit: String },
    Collect { manifest: String },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Profile {
    pub(crate) path: String,
    pub(crate) sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Context {
    pub(crate) path: String,
    pub(crate) raw: Vec<u8>,
    pub(crate) sha256: String,
    pub(crate) semantic_digest: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Original {
    pub(crate) path: String,
    pub(crate) sha256: String,
}
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
pub(crate) fn artifact_path(path: &str) -> Result<(), String> {
    relative_path(path)?;
    if path.starts_with(".chrono-harness/state/") {
        Ok(())
    } else {
        Err("check artifact must be in .chrono-harness/state/".into())
    }
}
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
impl InputRequest {
    pub(crate) fn validate(&self, config: &crate::Config) -> Result<(), String> {
        if self.schema != "chrono-check-input-request/v1"
            || self.source != "local"
            || !self.host_root.is_absolute()
            || self.native_artifacts.is_some()
            || self.prepared.is_some()
        {
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
        if initial && let Some(profile) = &self.initial_profile {
            if self.selection != Selection::All {
                return Err("initial inventory requires a bare check".into());
            }
            return Ok(&profile.path);
        }
        Ok(&self.profile)
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Action {
    pub(crate) operation: String,
    #[serde(rename = "tool")]
    _tool: String,
    pub(crate) argv: Vec<String>,
}
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
pub(crate) fn original(root: &Path, path: &str) -> Result<Original, String> {
    artifact_path(path)?;
    let digest = file_identity(&no_symlink_parents(root, path)?)?.0;
    Ok(Original {
        path: path.into(),
        sha256: digest,
    })
}
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
    if let Some(inventory) = crate::retention::Inventory::adopted(&root)? {
        if inventory.accepts("check-original", relative)? {
            let publication = inventory.create_referenced(
                "check-original",
                relative,
                "publication-recheck",
                json!({"publication":"started","outcome":"not-established"}),
            )?;
            publication.replace_file(&file, &raw)?;
            publication.complete(outcome)?;
            return Ok(relative.into());
        }
    }
    let mut output = fs::File::create_new(&file).map_err(|e| e.to_string())?;
    output
        .write_all(&raw)
        .and_then(|()| output.sync_all())
        .map_err(|e| e.to_string())?;
    Ok(relative.into())
}
