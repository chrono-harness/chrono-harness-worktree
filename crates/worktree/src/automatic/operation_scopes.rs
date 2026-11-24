//! Concurrent managed commands declare the checkout paths, index and refs they use.
//! Each live declaration is protected by an inherited kernel capability.
use crate::{
    document::{no_symlink_parents, relative_path},
    ownership::{DirectoryLease, Lease},
    start::CheckoutIdentity,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Scope {
    pub reads: Vec<String>,
    pub writes: Vec<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Claim {
    operation: String,
    token: String,
    reads: Vec<PathBuf>,
    writes: Vec<PathBuf>,
}

fn resolve(checkout: &CheckoutIdentity, resource: &str) -> Result<PathBuf, String> {
    match resource {
        "checkout" => Ok(checkout.top.clone()),
        "git:index" => Ok(checkout.metadata.join("index")),
        "git:head" => Ok(checkout.metadata.join("HEAD")),
        "git:branch" => {
            if checkout.branch == "HEAD" {
                return Ok(checkout.metadata.join("HEAD"));
            }
            if !checkout.branch.starts_with("refs/") {
                return Err("expected an attached Git ref".into());
            }
            no_symlink_parents(&checkout.common, &checkout.branch)
        }
        _ if resource.starts_with("git:refs/") => {
            no_symlink_parents(&checkout.common, &resource[4..])
        }
        _ if resource.starts_with("path:") => {
            let name = &resource[5..];
            if name.split('/').any(|part| {
                part.is_empty() || part == "." || part == ".." || part.eq_ignore_ascii_case(".git")
            }) {
                return Err(format!("invalid checkout resource {resource:?}"));
            }
            no_symlink_parents(&checkout.top, name)
        }
        _ => Err(format!("unknown managed command resource {resource:?}")),
    }
}

/// Compare the actual resolved resources from two independently running commands.
#[doc(hidden)]
pub fn conflicting_paths(
    reads: &[PathBuf],
    writes: &[PathBuf],
    other_reads: &[PathBuf],
    other_writes: &[PathBuf],
) -> bool {
    let overlap = |left: &Path, right: &Path| left.starts_with(right) || right.starts_with(left);
    writes.iter().any(|p| {
        other_reads
            .iter()
            .chain(other_writes)
            .any(|q| overlap(p, q))
    }) || other_writes
        .iter()
        .any(|p| reads.iter().any(|q| overlap(p, q)))
}

impl Claim {
    fn resolve(
        operation: &str,
        token: &str,
        scope: Option<&Scope>,
        checkout: &CheckoutIdentity,
    ) -> Result<Self, String> {
        let addresses = |names: &[String]| {
            names
                .iter()
                .map(|name| resolve(checkout, name))
                .collect::<Result<Vec<_>, _>>()
        };
        let (reads, writes) = match scope {
            Some(scope) => (addresses(&scope.reads)?, addresses(&scope.writes)?),
            None => (
                vec![],
                vec![
                    checkout.top.clone(),
                    checkout.metadata.clone(),
                    checkout.common.join("refs"),
                ],
            ),
        };
        Ok(Self {
            operation: operation.into(),
            token: token.into(),
            reads,
            writes,
        })
    }
}

fn record_name(bytes: &[u8]) -> String {
    format!("use-{}.json", crate::document::sha256(bytes))
}

fn sweep(dir: &Path, requested: Option<&Claim>) -> Result<usize, String> {
    let mut expired = 0;
    let pending = dir.join("publishing");
    match fs::symlink_metadata(&pending) {
        Ok(m) if m.is_file() => fs::remove_file(&pending).map_err(|e| e.to_string())?,
        Err(e) if e.kind() == io::ErrorKind::NotFound => (),
        _ => return Err("managed scope staging identity changed".into()),
    }
    for item in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let item = item.map_err(|e| e.to_string())?;
        let name = item.file_name();
        let Some(name) = name
            .to_str()
            .filter(|s| s.starts_with("use-") && s.ends_with(".json"))
        else {
            continue;
        };
        if !item.file_type().map_err(|e| e.to_string())?.is_file() {
            return Err("managed scope record is not a regular file".into());
        }
        let path = item.path();
        let bytes = fs::read(&path).map_err(|e| e.to_string())?;
        if record_name(&bytes) != name {
            return Err(format!(
                "managed scope record identity differs at {}",
                path.display()
            ));
        }
        let previous: Claim = serde_json::from_slice(&bytes)
            .map_err(|e| format!("managed scope record {}: {e}", path.display()))?;
        if let Some(lease) = Lease::acquire(&path, None, false, true, None)? {
            lease.stable()?;
            fs::remove_file(&path).map_err(|e| e.to_string())?;
            expired += 1;
        } else if requested.is_some_and(|request| {
            conflicting_paths(
                &request.reads,
                &request.writes,
                &previous.reads,
                &previous.writes,
            )
        }) {
            return Err(format!(
                "managed operation overlaps a live read/write scope held by {}",
                previous.operation
            ));
        }
    }
    Ok(expired)
}

pub(super) struct Active {
    directory: PathBuf,
    path: PathBuf,
    lease: Option<Lease>,
}

impl Drop for Active {
    fn drop(&mut self) {
        let Some(owned) = self.lease.take() else {
            return;
        };
        let identity = owned.id().to_owned();
        drop(owned);
        // A surviving child can still hold the inode after the launcher returns.
        if let Ok(Some(_gate)) = DirectoryLease::acquire(&self.directory, true, None)
            && let Ok(Some(lease)) = Lease::acquire(&self.path, Some(&identity), false, true, None)
            && lease.stable().is_ok()
        {
            let _ = fs::remove_file(&self.path);
        }
    }
}

pub(super) fn enter(
    directory: &Path,
    registered: &BTreeMap<String, Scope>,
    checkout: impl FnOnce() -> Result<CheckoutIdentity, String>,
    operation: &str,
    token: &str,
) -> Result<Option<Active>, String> {
    let directory = directory.join("operation-scopes");
    if registered.is_empty() && !directory.try_exists().map_err(|e| e.to_string())? {
        return Ok(None);
    }
    relative_path(token)?;
    let requested = Claim::resolve(operation, token, registered.get(operation), &checkout()?)?;
    fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let _gate = DirectoryLease::blocking(&directory, true)?;
    sweep(&directory, Some(&requested))?;
    let bytes = serde_json::to_vec(&requested).map_err(|e| e.to_string())?;
    let path = directory.join(record_name(&bytes));
    // This sole staging name is protected by the short admission lock. A terminated
    // publisher can leave at most one partial record, recoverable by maintain.
    let pending = directory.join("publishing");
    let mut body = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&pending)
        .map_err(|e| format!("managed scope staging: {e}"))?;
    body.write_all(&bytes).map_err(|e| e.to_string())?;
    body.flush().map_err(|e| e.to_string())?;
    fs::hard_link(&pending, &path).map_err(|e| e.to_string())?;
    fs::remove_file(&pending).map_err(|e| e.to_string())?;
    let lease = Lease::acquire(&path, None, false, true, None)?
        .ok_or("new semantic scope was already held")?;
    Ok(Some(Active {
        directory,
        path,
        lease: Some(lease),
    }))
}

pub(super) fn cleanup(directory: &Path) -> Result<usize, String> {
    let dir = directory.join("operation-scopes");
    match fs::symlink_metadata(&dir) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(e.to_string()),
        Ok(m) if !m.is_dir() => return Err("managed scope directory identity changed".into()),
        Ok(_) => (),
    }
    let Some(_gate) = DirectoryLease::acquire(&dir, true, None)? else {
        return Ok(0);
    };
    sweep(&dir, None)
}
