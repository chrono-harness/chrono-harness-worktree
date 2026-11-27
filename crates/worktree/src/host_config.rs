//! Read only the fixed host documents consumed by the worktree. An executable
//! identity selector uses its registered program as data, never as a command.
use vstd::prelude::*;

verus! {
/// A selected platform policy must be a direct supported host document.
pub(crate) fn direct_target(selector_field: bool, schema_version: Option<u64>) -> (valid: bool)
    ensures valid == (!selector_field && (schema_version == Some(3u64) || schema_version == Some(4u64))),
{
    !selector_field && matches!(schema_version, Some(3) | Some(4))
}
}

#[cfg(not(verus_keep_ghost))]
use crate::document::{file_identity, json, relative_path, valid_digest};
#[cfg(not(verus_keep_ghost))]
use serde::Deserialize;
#[cfg(not(verus_keep_ghost))]
use serde_json::Value;
#[cfg(not(verus_keep_ghost))]
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[cfg(not(verus_keep_ghost))]
pub(crate) struct Resolved {
    pub(crate) selected: String,
    pub(crate) values: BTreeMap<String, Value>,
}
#[cfg(not(verus_keep_ghost))]
impl Resolved {
    pub(crate) fn into_selected(mut self) -> Result<Value, String> {
        self.values
            .remove(&self.selected)
            .ok_or("missing selected configuration".into())
    }
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Selector {
    schema: String,
    platforms: BTreeMap<String, Value>,
}
#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Identity {
    program: String,
    configurations: Vec<Target>,
}
#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Target {
    sha256: String,
    path: String,
    #[serde(default)]
    registry_path: Option<String>,
}

#[cfg(not(verus_keep_ghost))]
fn host_path(path: &str, entry: Option<&str>) -> Result<(), String> {
    relative_path(path)?;
    if !path.starts_with(".chrono-harness/") || Some(path) == entry {
        return Err(format!("invalid host selector path: {path}"));
    }
    Ok(())
}
#[cfg(not(verus_keep_ghost))]
fn select(
    entry: &str,
    policy: &Value,
) -> Result<Option<(String, Option<(String, String)>)>, String> {
    let Some(schema) = policy["schema"]
        .as_str()
        .filter(|s| s.starts_with("chrono-git-configs/"))
    else {
        return Ok(None);
    };
    let has_identity = match schema {
        "chrono-git-configs/v1" => false,
        "chrono-git-configs/v2" | "chrono-git-configs/v3" => true,
        _ => return Err(format!("unsupported Git config selector schema {schema}")),
    };
    host_path(entry, None)?;
    let selector: Selector = serde_json::from_value(policy.clone()).map_err(|e| e.to_string())?;
    let platform = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
    let mut result = None;
    for (name, host) in selector.platforms {
        if name.is_empty() || name.trim() != name {
            return Err("Git selector platform must be nonempty and literal".into());
        }
        if !has_identity {
            let path = host.as_str().ok_or("missing platform policy path")?;
            host_path(path, Some(entry))?;
            if platform == name {
                result = Some((path.into(), None));
            }
            continue;
        }
        let identity: Identity = serde_json::from_value(host.clone()).map_err(|e| e.to_string())?;
        if !Path::new(&identity.program).is_absolute()
            || identity.program.contains(['\0', '\n', '\r'])
            || identity.configurations.is_empty()
        {
            return Err(format!(
                "invalid Git executable identity registration for {name}"
            ));
        }
        let mut paths = BTreeSet::new();
        let mut hashes = BTreeSet::new();
        let requires_registry = selector.schema == "chrono-git-configs/v3";
        let rows = host["configurations"]
            .as_array()
            .ok_or("missing configurations")?;
        for (target, raw) in identity.configurations.iter().zip(rows) {
            if !valid_digest(&target.sha256)
                || !paths.insert(target.path.as_str())
                || !hashes.insert(&target.sha256)
                || raw.get("registry_path").is_some() != requires_registry
                || target.registry_path.is_some() != requires_registry
            {
                return Err(format!("invalid or duplicate policy identity for {name}"));
            }
            host_path(&target.path, Some(entry))?;
        }
        if identity
            .configurations
            .iter()
            .filter_map(|t| t.registry_path.as_deref())
            .any(|path| !paths.contains(path))
        {
            return Err("Git selector registry_path must name a registered policy".into());
        }
        if name == platform {
            let (digest, _) = file_identity(Path::new(&identity.program))?;
            let target = identity
                .configurations
                .iter()
                .find(|t| t.sha256 == digest)
                .ok_or_else(|| format!("no {name} policy for Git {}", identity.program))?;
            result = Some((target.path.to_owned(), Some((identity.program, digest))));
        }
    }
    result
        .map(Some)
        .ok_or_else(|| format!("no Git policy registered for {platform}"))
}
#[cfg(not(verus_keep_ghost))]
fn reference<'a>(rows: &'a Value, key: &Value) -> Result<&'a Value, String> {
    let key = key
        .as_str()
        .filter(|id| !id.is_empty())
        .ok_or("missing executable binding ID")?;
    let mut matches = rows
        .as_array()
        .ok_or("missing executable binding registrations")?
        .iter()
        .filter(|row| row["id"] == key);
    let row = matches.next().ok_or("unregistered executable binding")?;
    if matches.next().is_some() {
        return Err("ambiguous executable binding".into());
    }
    Ok(row)
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn read(
    entry: &str,
    mut read: impl FnMut(&str) -> Result<Vec<u8>, String>,
) -> Result<Resolved, String> {
    relative_path(entry)?;
    let original = json(&read(entry)?)?;
    let target = select(entry, &original)?;
    let mut values = BTreeMap::from([(entry.to_owned(), original)]);
    let selected = if let Some((path, identity)) = target {
        let policy = json(&read(&path)?)?;
        if !direct_target(
            policy.get("schema").is_some(),
            policy["schema_version"].as_u64(),
        ) {
            return Err("selector requires a direct schema 3 or 4 policy".into());
        }
        if let Some((program, digest)) = identity {
            let link = &policy["facts_git"];
            let tool = reference(&policy["tools"], &link["tool"])?;
            let input = reference(&policy["environment"]["inputs"], &link["input"])?;
            if tool["program"] != program
                || tool["resolution"] != "PATH-once"
                || input["location"] != program
                || input["presence"] != "present"
                || input["sha256"] != digest
            {
                return Err("selected policy executable identity differs from selector".into());
            }
        }
        values.insert(path.clone(), policy);
        path
    } else {
        entry.to_owned()
    };
    Ok(Resolved { selected, values })
}
