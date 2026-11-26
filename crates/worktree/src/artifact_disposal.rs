//! Explicit directory disposal shared by legacy checkout cleanup and lifecycle cleanup.
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
use super::{has_prefix, prefix};
#[cfg(not(verus_keep_ghost))]
use crate::verified_owner_publication::has_prefix;
#[cfg(not(verus_keep_ghost))]
use vstd::prelude::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

/// Registered directory names can have multiple trailing slashes.
pub fn disposal_directory(name: &[u8]) -> (directory: &[u8])
    ensures
        directory@ == name@.subrange(0, directory@.len() as int),
        directory@.len() <= name@.len(),
        directory@.len() == 0 || directory@.last() != 47u8,
        forall|i: int| directory@.len() <= i < name@.len() ==> name@[i] == 47u8,
{
    let mut end = name.len();
    while end > 0 && name[end - 1] == 47u8
        invariant
            end <= name.len(),
            forall|i: int| end <= i < name.len() ==> name@[i] == 47u8,
        decreases end,
    { end -= 1; }
    name.split_at(end).0
}

pub fn contains_source(directory: &[u8], path: &[u8]) -> (retained: bool)
    ensures retained <==> (prefix(path@, directory@)
        && (path@.len() == directory@.len() || path@[directory@.len() as int] == 47u8)),
{
    has_prefix(path, directory)
        && (path.len() == directory.len() || path[directory.len()] == 47u8)
}

/// Git errors carry their original type and value through either query.
pub fn inventory_contains_source<E, F>(index: Result<Vec<u8>, E>, deletions: F)
    -> (out: Result<bool, E>)
    where F: FnOnce() -> Result<Vec<u8>, E>,
    requires (index is Ok && index->Ok_0@.len() == 0) ==> deletions.requires(()),
    ensures match index {
        Err(error) => out == Err::<bool, E>(error),
        Ok(bytes) => if bytes@.len() != 0 { out == Ok::<bool, E>(true) }
            else { exists|observed: Result<Vec<u8>, E>| #[trigger] deletions.ensures((), observed)
                && out == match observed {
                    Err(error) => Err::<bool, E>(error),
                    Ok(data) => Ok::<bool, E>(data@.len() != 0),
                } },
    },
{
    match index {
        Err(error) => Err(error),
        Ok(bytes) => {
            if !bytes.is_empty() { return Ok(true); }
            match deletions() {
                Err(error) => Err(error),
                Ok(data) => Ok(!data.is_empty()),
            }
        },
    }
}
}

#[cfg(not(verus_keep_ghost))]
use crate::document::{no_symlink_parents, relative_path};
#[cfg(not(verus_keep_ghost))]
use crate::start::Runner;
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::os::unix::fs::{FileTypeExt, MetadataExt};
#[cfg(not(verus_keep_ghost))]
use std::{
    fs,
    path::{Path, PathBuf},
};

#[cfg(not(verus_keep_ghost))]
pub(crate) fn registered(names: &[String], endpoints: &[&Value]) -> Result<(), String> {
    for name in names {
        for host in endpoints {
            if !host["artifacts"]
                .as_array()
                .ok_or("missing artifact registrations")?
                .iter()
                .any(|a| a["path"] == *name && a["tracked"] == false)
            {
                return Err(
                    "disposal directory is not an artifact in both endpoint registries".into(),
                );
            }
        }
    }
    Ok(())
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn paths(
    r: &mut Runner,
    target: &Path,
    head: &str,
    names: &[String],
) -> Result<Vec<PathBuf>, String> {
    if names.is_empty() {
        return Ok(vec![]);
    }
    let tree = r.tree(target, head)?;
    // Staged files are also source, including additions within an artifact prefix.
    let index = crate::start::paths(r.git(target, &["ls-files", "-z"])?)?;
    names
        .iter()
        .map(|name| {
            let directory = disposal_directory(name.as_bytes());
            if tree
                .keys()
                .map(String::as_str)
                .chain(index.iter().map(String::as_str))
                .any(|p| contains_source(directory, p.as_bytes()))
            {
                return Err(format!("disposal directory contains tracked paths: {name}"));
            }
            let path = no_symlink_parents(target, &name[..directory.len()])?;
            match fs::symlink_metadata(&path) {
                Ok(m) if m.is_dir() => Ok(path),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(path),
                Err(e) => Err(e.to_string()),
                Ok(_) => Err(format!("disposal requires a physical directory: {name}")),
            }
        })
        .collect()
}

#[cfg(not(verus_keep_ghost))]
#[derive(Default, serde::Serialize)]
pub struct ArtifactFootprint {
    pub file_entries: u64,
    pub logical_bytes: u64,
    pub allocated_bytes: Option<u64>,
}

#[cfg(not(verus_keep_ghost))]
fn measure(root: &Path, allow_sockets: bool) -> Result<(ArtifactFootprint, u64), String> {
    let mut pending = vec![root.to_owned()];
    let mut footprint = ArtifactFootprint::default();
    let mut blocks = 0u64;
    let mut sockets = 0u64;
    while let Some(path) = pending.pop() {
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.to_string()),
        };
        blocks = blocks
            .checked_add(metadata.blocks())
            .ok_or("artifact allocation overflow")?;
        let kind = metadata.file_type();
        if kind.is_dir() {
            for entry in fs::read_dir(&path).map_err(|error| error.to_string())? {
                pending.push(entry.map_err(|error| error.to_string())?.path());
            }
            continue;
        }
        if allow_sockets && kind.is_socket() {
            sockets = sockets
                .checked_add(1)
                .ok_or("artifact socket count overflow")?;
        } else if !kind.is_file() && !kind.is_symlink() {
            return Err("artifact contains a special file; preserve it".into());
        }
        footprint.file_entries = footprint
            .file_entries
            .checked_add(1)
            .ok_or("artifact count overflow")?;
        footprint.logical_bytes = footprint
            .logical_bytes
            .checked_add(metadata.len())
            .ok_or("artifact byte count overflow")?;
    }
    footprint.allocated_bytes = Some(
        blocks
            .checked_mul(512)
            .ok_or("artifact allocation overflow")?,
    );
    Ok((footprint, sockets))
}

#[cfg(not(verus_keep_ghost))]
pub fn artifact_footprint(path: &Path) -> Result<ArtifactFootprint, String> {
    measure(path, false).map(|(footprint, _)| footprint)
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn observed_artifact_footprint(path: &Path) -> Result<(ArtifactFootprint, u64), String> {
    measure(path, true)
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn identity(path: &Path) -> Result<String, String> {
    let metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    let kind = metadata.file_type();
    let label = if kind.is_dir() {
        "directory"
    } else if kind.is_file() {
        "file"
    } else if kind.is_symlink() {
        "link"
    } else {
        "special"
    };
    Ok(format!("{}:{}:{label}", metadata.dev(), metadata.ino()))
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn dispose_entry(
    root: &Path,
    relative: &str,
    expected: &str,
    mut before_effect: impl FnMut() -> Result<(), String>,
) -> Result<(), String> {
    relative_path(relative)?;
    let relative = Path::new(relative);
    let parent = relative.parent().ok_or("artifact parent")?;
    let resolve = || -> Result<PathBuf, String> {
        let directory = if parent.as_os_str().is_empty() {
            root.to_owned()
        } else {
            no_symlink_parents(root, parent.to_str().ok_or("artifact parent UTF8")?)?
        };
        Ok(directory.join(relative.file_name().ok_or("artifact name")?))
    };
    let path = resolve()?;
    let kind = fs::symlink_metadata(&path)
        .map_err(|error| error.to_string())?
        .file_type();
    if !kind.is_file() && !kind.is_dir() && !kind.is_symlink() {
        return Err("artifact contains a special file; preserve it".into());
    }
    before_effect()?;
    if identity(&resolve()?)? != expected {
        return Err("artifact identity changed before effect".into());
    }
    if kind.is_dir() {
        fs::remove_dir(path)
    } else {
        fs::remove_file(path)
    }
    .map_err(|error| error.to_string())
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn publish_entry(source: &Path, destination: &Path) -> Result<(), String> {
    use rustix::fs::{CWD, RenameFlags, renameat_with};
    renameat_with(CWD, source, CWD, destination, RenameFlags::NOREPLACE)
        .map_err(|error| error.to_string())
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn source_safe(r: &mut Runner, root: &Path, relative: &str) -> Result<(), String> {
    relative_path(relative)?;
    match fs::symlink_metadata(root.join(".git")) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.to_string()),
        Ok(_) => (),
    }
    // Staged deletions and rename sources no longer appear in ls-files.
    let tracked = r.literal_inventory(
        root,
        &["--literal-pathspecs", "ls-files", "-z", "--", relative],
    );
    if inventory_contains_source(tracked, || {
        r.literal_inventory(
            root,
            &[
                "--literal-pathspecs",
                "diff",
                "--cached",
                "--name-only",
                "-z",
                "--no-renames",
                "--ignore-submodules=none",
                "--diff-filter=D",
                "--",
                relative,
            ],
        )
    })? {
        return Err(format!(
            "retained output contains committed/index source: {relative}"
        ));
    }
    Ok(())
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn dispose(
    target: &Path,
    names: &[String],
    paths: &[PathBuf],
    report: &mut Value,
    mut before_effect: impl FnMut() -> Result<(), String>,
) -> Result<(), String> {
    let state = |path: &Path, name: &str| match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok("already-absent"),
        Err(e) => Err(e.to_string()),
        Ok(m) if m.is_dir() => Ok("attempted-unverified"),
        Ok(_) => Err(format!("disposal requires a physical directory: {name}")),
    };
    let mut retained = Vec::new();
    for (name, path) in names.iter().zip(paths) {
        no_symlink_parents(target, name.trim_end_matches('/'))?;
        let mut observed = state(path, name)?;
        // An absent output has no removal effect. Recheck both ownership and
        // the physical path before every actual removal.
        if observed != "already-absent" {
            before_effect()?;
            no_symlink_parents(target, name.trim_end_matches('/'))?;
            observed = state(path, name)?;
        }
        let measured = report["automatic_cleanup"] == true;
        let mut effect = json!({"path":name,"status":observed});
        if measured {
            effect["byte_measure"] = json!("literal-entry-lengths");
            let footprint = artifact_footprint(path)?;
            effect["bytes_before"] = json!(footprint.logical_bytes);
            effect["footprint_before"] = json!(footprint);
            effect["allocated_measure"] = json!("lstat-blocks-times-512-including-directories");
            effect["physical_release_bytes"] = Value::Null;
            effect["bytes_after"] = json!(if observed == "already-absent" {
                Some(0)
            } else {
                None::<u64>
            });
        }
        report["artifact_disposals"]
            .as_array_mut()
            .ok_or("missing artifact disposal report")?
            .push(effect);
        if observed == "already-absent" {
            continue;
        }
        let _retention = match crate::removal::Guard::one(target, name.trim_end_matches('/')) {
            Ok(guard) => guard,
            Err(error) => {
                let effect = report["artifact_disposals"]
                    .as_array_mut()
                    .unwrap()
                    .last_mut()
                    .unwrap();
                effect["status"] = json!("preserved");
                effect["error"] = json!(error);
                retained.push(format!("{name}: {error}"));
                continue;
            }
        };
        if let Some(retention) = &_retention {
            retention.stable()?;
        }
        // Internal links are unlinked; their external targets are never traversed.
        if let Err(error) = fs::remove_dir_all(path) {
            let effect = report["artifact_disposals"]
                .as_array_mut()
                .unwrap()
                .last_mut()
                .unwrap();
            effect["status"] = json!("failed-partial-effects-possible");
            effect["error"] = json!(error.to_string());
            if measured {
                match artifact_footprint(path) {
                    Ok(remaining) => {
                        effect["bytes_after"] = json!(remaining.logical_bytes);
                        effect["footprint_after"] = json!(remaining);
                    }
                    Err(observation_error) => {
                        effect["remaining_measure_error"] = json!(observation_error)
                    }
                }
            }
            return Err(format!("artifact disposal {name}: {error}"));
        }
        match fs::symlink_metadata(path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.to_string()),
            Ok(_) => return Err(format!("disposed artifact still exists: {name}")),
        }
        let effect = report["artifact_disposals"]
            .as_array_mut()
            .unwrap()
            .last_mut()
            .unwrap();
        effect["status"] = json!("verified-absent");
        if measured {
            effect["bytes_after"] = json!(0);
            effect["footprint_after"] = json!(artifact_footprint(path)?);
        }
    }
    if retained.is_empty() {
        Ok(())
    } else {
        Err(retained.join("; "))
    }
}
