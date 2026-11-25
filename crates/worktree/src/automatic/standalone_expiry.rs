//! Roll the registered worktree report directory when no shared original owner is installed.
//! Current Git source, inherited directory leases and enrollment references take precedence.
use super::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    time::{SystemTime, UNIX_EPOCH},
};

struct Original {
    identity: String,
    bytes: u64,
    written: u64,
    dependencies: Vec<String>,
}

fn name_token(name: &str) -> Option<&str> {
    let (token, _) = name.split_once(".json")?;
    let mut fields = token.rsplitn(4, '-');
    let sequence = fields.next()?;
    let clock = fields.next()?;
    let pid = fields.next()?;
    let operation = fields.next()?;
    if operation.is_empty()
        || [sequence, clock, pid]
            .iter()
            .any(|v| v.is_empty() || !v.bytes().all(|b| b.is_ascii_digit()))
    {
        return None;
    }
    Some(token)
}

fn capture(root: &Path, relative: &str, dependencies: Vec<String>) -> Result<Original, String> {
    let path = no_symlink_parents(root, relative)?;
    let metadata = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
    if !metadata.is_file() && !metadata.is_dir() {
        return Err(format!(
            "report member is not a physical file/directory: {relative}"
        ));
    }
    let written = metadata
        .modified()
        .map_err(|e| e.to_string())?
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    Ok(Original {
        identity: crate::artifact_disposal::identity(&path)?,
        bytes: if metadata.is_file() {
            metadata.len()
        } else {
            crate::artifact_disposal::artifact_footprint(&path)?.logical_bytes
        },
        written,
        dependencies,
    })
}

fn recognized_report(root: &Path, prefix: &str, name: &str, token: &str, value: &Value) -> bool {
    value["schema"] == "chrono-worktree-report/v1"
        && value["source_root"] == value!(root)
        && value["report_path"] == format!("{prefix}{name}")
        && value["lock_reason"] == token
        && value["operation"]
            .as_str()
            .is_some_and(|op| token.strip_prefix(op).is_some_and(|s| s.starts_with('-')))
}

fn use_binding(value: &Value) -> Option<crate::verified_lifecycle::ManagedUseBinding<'_>> {
    let process = &value["managed_process_original"];
    Some(crate::verified_lifecycle::ManagedUseBinding {
        schema: value["schema"].as_str()?,
        checkout: value["path"].as_str()?,
        operation: value["operation"].as_str()?,
        report_path: value["report_path"].as_str()?,
        process_path: process["path"].as_str()?,
        joined: process["joined"].as_bool()?,
    })
}

/// A sidecar has the report's actual receipt, or its own identity-bound operation record.
/// Undeclared foreign files in either business directory never enter the disposal set.
fn related_file(
    root: &Path,
    report_path: &str,
    report: &Value,
    state: &str,
    path: &str,
) -> Result<bool, String> {
    let p = no_symlink_parents(root, path)?;
    let name = Path::new(report_path)
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("invalid report name")?;
    if path == format!("{report_path}.inventory.jsonl") {
        let binding = &report["complete_inventory"];
        if binding["path"] != path {
            return Ok(false);
        }
        let digest = binding["receipt"]["sha256"]
            .as_str()
            .ok_or("inventory receipt lacks digest")?;
        let (actual, bytes) = crate::document::file_identity(&p)?;
        return Ok(actual == digest && binding["byte_length"] == bytes);
    }
    if path == format!("{state}birth-{name}") {
        let value = crate::document::json(&fs::read(&p).map_err(|e| e.to_string())?)?;
        return Ok(value["schema"] == "chrono-worktree-report/v1"
            && value["report_path"] == report_path
            && value["source_root"] == value!(root));
    }
    if !path.starts_with(&format!("{report_path}."))
        && !path
            .strip_prefix(state)
            .is_some_and(|s| !s.contains('/') && s.ends_with(name))
    {
        return Ok(false);
    }
    for binding in [
        "managed_use_receipt",
        "managed_use_intent",
        "main_cache_release",
        "cache_intent",
        "terminal_receipt",
    ] {
        if report[binding]["path"] == path {
            return Ok(report[binding]["sha256"].as_str().is_some_and(|digest| {
                crate::document::file_identity(&p).is_ok_and(|(hash, _)| hash == digest)
            }));
        }
    }
    let value = crate::document::json(&fs::read(&p).map_err(|e| e.to_string())?)?;
    if path == format!("{state}use-intent-{name}") {
        let result = &report["managed_use_receipt"];
        let result_path = format!("{state}use-{name}");
        if result["path"] != result_path || report["managed_process_original"]["joined"] != true {
            return Ok(false);
        }
        let result_file = no_symlink_parents(root, &result_path)?;
        let bytes = match fs::read(&result_file) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(error.to_string()),
        };
        if result["sha256"] != crate::document::sha256(&bytes) {
            return Ok(false);
        }
        let original = crate::document::json(&bytes)?;
        let (Some(intent), Some(completed)) = (use_binding(&value), use_binding(&original)) else {
            return Ok(false);
        };
        return Ok(
            original["managed_process_original"] == report["managed_process_original"]
                && crate::verified_lifecycle::binds_use_intent(
                    &intent,
                    &completed,
                    report_path,
                    &format!("{report_path}.process"),
                ),
        );
    }
    if value["report_path"] == report_path
        && value["source_root"] == value!(root)
        && value["schema"]
            .as_str()
            .is_some_and(|s| s.starts_with("chrono-worktree-"))
    {
        return Ok(true);
    }
    Ok(false)
}
pub(super) fn expire(
    r: &mut Runner,
    token: &str,
    inputs: &PolicyInputs,
    observation: &Observation<'_>,
    current: &Value,
) -> Result<Option<Value>, String> {
    let root = &inputs.policy.coordinator_root;
    if root
        .join(crate::retention::POLICY_PATH)
        .try_exists()
        .map_err(|e| e.to_string())?
    {
        return Ok(None);
    }
    let keep = inputs
        .policy
        .terminal_history
        .as_ref()
        .map_or(86_400, |policy| policy.keep_seconds);
    let prefix = r.config.report_directory.clone();
    let state = &inputs.policy.state_directory;
    let directory = no_symlink_parents(root, prefix.trim_end_matches('/'))?;
    // Only our own shared lease is handed over. If any other invocation is live
    // the exclusive attempt fails and the next scheduled round handles expiry.
    drop(r.report_guard.take());
    let exclusive = crate::ownership::DirectoryLease::acquire(&directory, true, None)?;
    let Some(lease) = exclusive else {
        r.report_guard = Some(crate::ownership::DirectoryLease::shared(&directory)?);
        return Ok(Some(
            value!({"status":"active-publishers","removed_files":0}),
        ));
    };
    let result = expire_locked(
        r,
        token,
        inputs,
        observation,
        current,
        keep,
        &prefix,
        state,
        &directory,
        &lease,
    );
    drop(lease);
    r.report_guard = Some(crate::ownership::DirectoryLease::shared(&directory)?);
    result
}
fn expire_locked(
    r: &mut Runner,
    token: &str,
    inputs: &PolicyInputs,
    observation: &Observation<'_>,
    current: &Value,
    keep: u64,
    prefix: &str,
    state: &str,
    directory: &Path,
    lease: &crate::ownership::DirectoryLease,
) -> Result<Option<Value>, String> {
    let root = &inputs.policy.coordinator_root;
    let mut protected = BTreeSet::new();
    protected.insert(
        current["report_path"]
            .as_str()
            .ok_or("missing current report path")?
            .to_owned(),
    );
    let mut incomplete = false;
    let paths = observation.enrollment_paths_observed(|_| incomplete = true)?;
    if incomplete {
        return Err("incomplete enrollment inventory; preserve reports".into());
    }
    for path in paths {
        let owner = Manager::open_metadata(
            r,
            root,
            &inputs.config_path,
            &inputs.anchor_config_bytes,
            token,
            &path,
            false,
            false,
            false,
        )?
        .ok_or("enrollment unavailable")?;
        protected.extend(owner.original_paths(&owner.ledger)?);
    }
    let mut originals = BTreeMap::new();
    let mut report_members = Vec::new();
    for entry in fs::read_dir(&directory).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let Some(report_token) = name_token(&name) else {
            continue;
        };
        let path = format!("{prefix}{name}");
        let address = format!("{prefix}{report_token}.json");
        // These suffixes are allocated by the worktree publisher. The report directory
        // is its explicit business directory. A crash may leave any prefix of a report.
        let suffix = &name[report_token.len()..];
        if matches!(
            suffix,
            ".json"
                | ".json.intent.json"
                | ".json.fetch-intent.json"
                | ".json.inventory.jsonl"
                | ".json.process"
        ) && address != current["report_path"]
            && (entry.file_type().map_err(|e| e.to_string())?.is_file()
                || (suffix == ".json.process"
                    && entry.file_type().map_err(|e| e.to_string())?.is_dir()))
        {
            report_members.push((path.clone(), address));
        }
        if name != format!("{report_token}.json")
            || path == current["report_path"]
            || !entry.file_type().map_err(|e| e.to_string())?.is_file()
        {
            continue;
        }
        let bytes = fs::read(entry.path()).map_err(|e| e.to_string())?;
        // An interrupted publication has no terminal owner evidence. It never authorizes a sidecar.
        let Ok(report) = crate::document::json(&bytes) else {
            continue;
        };
        if !recognized_report(root, &prefix, &name, report_token, &report) {
            continue;
        }
        let mut dependencies = crate::recovery::original_paths(root, &report)?;
        if report["managed_use_intent"].is_null()
            && report["managed_use_receipt"]["path"].is_string()
        {
            dependencies.push(format!("{state}use-intent-{name}"));
        }
        let birth = format!("{state}birth-{name}");
        if root.join(&birth).try_exists().map_err(|e| e.to_string())? {
            dependencies.push(birth);
        }
        for dep in &dependencies {
            if originals.contains_key(dep) {
                continue;
            }
            let source = no_symlink_parents(root, dep)?;
            if !source.try_exists().map_err(|e| e.to_string())?
                || !fs::symlink_metadata(&source)
                    .map_err(|e| e.to_string())?
                    .is_file()
            {
                continue;
            }
            if related_file(root, &path, &report, state, dep)? {
                originals.insert(dep.clone(), capture(root, dep, vec![])?);
            }
        }
        originals.insert(path.clone(), capture(root, &path, dependencies)?);
    }
    for (path, parent) in report_members {
        if path != parent {
            if let Some(record) = originals.get_mut(&parent) {
                record.dependencies.push(path.clone());
            }
        }
        if !originals.contains_key(&path) {
            originals.insert(path.clone(), capture(root, &path, vec![])?);
        }
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    for (path, item) in &originals {
        if !crate::verified_lifecycle::expired_original(item.written, now, keep, false) {
            protected.insert(path.clone());
        }
    }
    let mut pending: Vec<_> = protected.iter().cloned().collect();
    while let Some(path) = pending.pop() {
        if let Some(record) = originals.get(&path) {
            for dependency in &record.dependencies {
                if protected.insert(dependency.clone()) {
                    pending.push(dependency.clone());
                }
            }
        }
    }
    let mut removed = 0;
    let mut bytes = 0u64;
    // Sidecars first. A failed subset remains attributable to its original report on retry.
    let mut expired = originals
        .iter()
        .filter(|(path, item)| {
            crate::verified_lifecycle::expired_original(
                item.written,
                now,
                keep,
                protected.contains(*path),
            )
        })
        .collect::<Vec<_>>();
    expired.sort_by_key(|(_, record)| !record.dependencies.is_empty());
    if !expired.is_empty() {
        crate::artifact_disposal::source_safe(r, root, prefix.trim_end_matches('/'))?;
        crate::artifact_disposal::source_safe(r, root, state.trim_end_matches('/'))?;
    }
    for (path, record) in expired {
        if record.identity.ends_with(":directory") {
            let physical = no_symlink_parents(root, path)?;
            crate::artifact_disposal::dispose(
                root,
                std::slice::from_ref(path),
                std::slice::from_ref(&physical),
                &mut value!({"artifact_disposals":[]}),
                || {
                    lease.stable()?;
                    if crate::artifact_disposal::identity(&physical)? != record.identity {
                        return Err("managed original directory identity changed".into());
                    }
                    Ok(())
                },
            )?;
        } else {
            crate::artifact_disposal::dispose_entry(root, path, &record.identity, || {
                lease.stable()
            })?;
        }
        removed += 1;
        bytes = bytes
            .checked_add(record.bytes)
            .ok_or("expired report byte count overflow")?;
    }
    Ok(Some(
        value!({"status":"rotated","keep_seconds":keep,"removed_files":removed,"removed_bytes":bytes}),
    ))
}
