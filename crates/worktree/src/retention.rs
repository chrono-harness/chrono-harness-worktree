//! Host-opted original custody. Without a host policy the worktree writes only its own files.
use vstd::prelude::*;

verus! {
/// The caller uses its own writer only when the host returns Ok(false).
pub(crate) fn publish_accepted<O, E, A, P>(owner: Option<O>, accepts: A, publish: P)
    -> (answer: Result<bool, E>)
    where A: FnOnce(&O) -> Result<bool, E>, P: FnOnce(&O) -> Result<bool, E>,
    requires forall|o: &O| accepts.requires((o,)),
        forall|o: &O| publish.requires((o,)),
    ensures match owner {
        None => answer == Ok::<bool, E>(false),
        Some(o) => exists|selected: Result<bool, E>| accepts.ensures((&o,), selected)
            && match selected {
                Ok(true) => publish.ensures((&o,), answer),
                Ok(false) => answer == Ok::<bool, E>(false),
                Err(error) => answer == Err::<bool, E>(error),
            },
    },
{
    let Some(owner) = owner else { return Ok(false); };
    match accepts(&owner) {
        Ok(true) => publish(&owner),
        Ok(false) => Ok(false),
        Err(error) => Err(error),
    }
}
}

#[cfg(not(verus_keep_ghost))]
use crate::retention_session::Session;
#[cfg(not(verus_keep_ghost))]
use serde::de::DeserializeOwned;
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::{
    cell::RefCell,
    collections::BTreeMap,
    fs,
    io::{ErrorKind, Write},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    rc::{Rc, Weak},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(not(verus_keep_ghost))]
pub(crate) const POLICY_PATH: &str = ".chrono-harness/retention.json";

#[cfg(not(verus_keep_ghost))]
thread_local! {
    static SESSIONS: RefCell<BTreeMap<(PathBuf, bool), Weak<RefCell<Session>>>> = RefCell::new(BTreeMap::new());
}

/// Forward active host-original capabilities whenever worktree starts a native child.
/// Duplicates share the original file descriptions, including after this process exits.
#[cfg(not(verus_keep_ghost))]
pub(crate) fn forward_scope() -> Result<crate::process_fds::Scope, String> {
    use std::os::fd::AsFd;
    let sessions = SESSIONS.with(|all| {
        all.borrow()
            .values()
            .filter_map(Weak::upgrade)
            .collect::<Vec<_>>()
    });
    let mut files = Vec::new();
    for session in sessions {
        files.extend(session.borrow_mut().descriptors()?);
    }
    let borrowed = files.iter().map(AsFd::as_fd).collect::<Vec<_>>();
    crate::process_fds::Scope::new(&borrowed)
}

#[cfg(not(verus_keep_ghost))]
#[derive(Clone)]
pub(crate) struct Inventory {
    root: PathBuf,
    session: Rc<RefCell<Session>>,
}

#[cfg(not(verus_keep_ghost))]
impl Inventory {
    pub(crate) fn adopted(root: &Path) -> Result<Option<Self>, String> {
        match fs::symlink_metadata(root.join(POLICY_PATH)) {
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.to_string()),
            Ok(_) => {}
        }
        let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
        let session = SESSIONS.with(|all| -> Result<_, String> {
            let mut all = all.borrow_mut();
            let key = (
                root.clone(),
                crate::maintenance_signal::RoundGuard::active(),
            );
            if let Some(existing) = all.get(&key).and_then(Weak::upgrade) {
                return Ok(existing);
            }
            all.retain(|_, weak| weak.strong_count() != 0);
            let mut session = Session::open(&root)?;
            session.call::<()>(json!({"operation":"explicit-read-scopes"}))?;
            let session = Rc::new(RefCell::new(session));
            all.insert(key, Rc::downgrade(&session));
            Ok(session)
        })?;
        Ok(Some(Self { root, session }))
    }

    fn call<T: DeserializeOwned>(&self, op: Value) -> Result<T, String> {
        self.session.borrow_mut().call(op)
    }
    fn fact<T: DeserializeOwned>(&self, fact: Value) -> Result<T, String> {
        self.call(json!({"operation":"inventory","fact":fact}))
    }
    fn publication<T: DeserializeOwned>(&self, action: Value) -> Result<T, String> {
        self.call(json!({"operation":"publication","action":action}))
    }
    pub(crate) fn dependencies_paths(
        &self,
        path: &str,
        originals: &[String],
    ) -> Result<(), String> {
        self.call(json!({"operation":"dependencies","path":path,"originals":originals}))
    }
    pub(crate) fn reference_paths(&self, role: &str, paths: &[String]) -> Result<(), String> {
        self.fact(json!({"action":"references","role":role,"paths":paths}))
    }
    pub(crate) fn prune_reference_paths(&self, role: &str, paths: &[String]) -> Result<(), String> {
        self.fact(json!({"action":"references","role":role,"paths":paths,"prune":true}))
    }
    pub(crate) fn published_original(
        &self,
        path: &str,
        sha256: &str,
        length: u64,
    ) -> Result<bool, String> {
        self.fact(json!({"action":"published","path":path,"sha256":sha256,"length":length}))
    }
    pub(crate) fn producer_status(&self, producer: &str, path: &str) -> Result<Value, String> {
        self.fact(json!({"action":"status","producer":producer,"path":path}))
    }
    pub(crate) fn accepts(&self, producer: &str, path: &str) -> Result<bool, String> {
        self.call(json!({"operation":"accepts","producer":producer,"path":path}))
    }
    pub(crate) fn retain_diagnostic_path(&self, path: &str) -> Result<(), String> {
        self.publication(json!({"kind":"retain-diagnostic","path":path}))
    }
    pub(crate) fn create_referenced(
        &self,
        producer: &str,
        path: &str,
        role: &str,
        outcome: Value,
    ) -> Result<Publication, String> {
        self.publication::<()>(json!({"kind":"create-referenced","path":path,"producer":producer,"reference":role,"outcome":outcome}))?;
        Ok(Publication::new(self.clone(), path))
    }
    pub(crate) fn resume_unfinished_directory(
        &self,
        producer: &str,
        path: &str,
    ) -> Result<Option<Publication>, String> {
        let selected: bool =
            self.publication(json!({"kind":"resume-directory","path":path,"producer":producer}))?;
        Ok(selected.then(|| Publication::new(self.clone(), path)))
    }
    fn relative(&self, path: &Path) -> Result<String, String> {
        let parent = fs::canonicalize(path.parent().ok_or("missing output parent")?)
            .map_err(|e| e.to_string())?;
        let path = parent.join(path.file_name().ok_or("missing output name")?);
        let relative = path
            .strip_prefix(&self.root)
            .map_err(|_| "output outside adopted host")?
            .to_str()
            .ok_or("output path UTF8")?;
        crate::document::relative_path(relative)?;
        Ok(relative.to_owned())
    }
}

#[cfg(not(verus_keep_ghost))]
pub(crate) struct Guard {
    owner: Inventory,
    close: Option<Value>,
}

#[cfg(not(verus_keep_ghost))]
impl Guard {
    fn new(owner: Inventory, close: Value) -> Self {
        Self {
            owner,
            close: Some(close),
        }
    }
}

#[cfg(not(verus_keep_ghost))]
impl Drop for Guard {
    fn drop(&mut self) {
        if let Some(close) = self.close.take() {
            // EOF releases all remaining capabilities if the owner has already exited.
            let _ = self.owner.publication::<()>(close);
        }
    }
}

#[cfg(not(verus_keep_ghost))]
pub(crate) type ReadScope = Option<Guard>;

#[cfg(not(verus_keep_ghost))]
fn scope(owner: Inventory, path: Option<&str>, inputs: bool) -> Result<Guard, String> {
    let token: u64 = owner.publication(json!({"kind":"scope","path":path,"inputs":inputs}))?;
    Ok(Guard::new(owner, json!({"kind":"end-scope","token":token})))
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn read_scope(root: &Path) -> Result<ReadScope, String> {
    Inventory::adopted(root)?
        .map(|owner| scope(owner, None, true))
        .transpose()
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn published_output_scope(p: &Publication) -> Result<ReadScope, String> {
    scope(p.owner.clone(), Some(&p.path), false).map(Some)
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn publication_read_scope(p: &Publication) -> Result<ReadScope, String> {
    scope(p.owner.clone(), Some(&p.path), true).map(Some)
}

#[cfg(not(verus_keep_ghost))]
fn read(root: &Path, path: &str, consumer: Option<&str>) -> Result<Option<Guard>, String> {
    let Some(owner) = Inventory::adopted(root)? else {
        return Ok(None);
    };
    let token: Option<u64> =
        owner.publication(json!({"kind":"read","path":path,"consumer":consumer}))?;
    Ok(token.map(|token| Guard::new(owner, json!({"kind":"release-read","token":token}))))
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn read_guard(root: &Path, path: &str) -> Result<Option<Guard>, String> {
    read(root, path, None)
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn consumer_read_guard(
    root: &Path,
    path: &str,
    consumer: &str,
) -> Result<Option<Guard>, String> {
    read(root, path, Some(consumer))
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn current_report_role(slot: &str) -> String {
    format!(
        "current-report:{}",
        crate::document::sha256(slot.as_bytes())
    )
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn reference_report(
    root: &Path,
    slot: &str,
    path: &str,
    inputs: &[String],
) -> Result<Option<Guard>, String> {
    let Some(owner) = Inventory::adopted(root)? else {
        return Ok(None);
    };
    let token: Option<u64> = owner
        .publication(json!({"kind":"reference-held","slot":slot,"path":path,"inputs":inputs}))?;
    Ok(token.map(|token| Guard::new(owner, json!({"kind":"release-read","token":token}))))
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn settle_report(root: &Path, slot: &str, path: &str) -> Result<(), String> {
    if let Some(owner) = Inventory::adopted(root)? {
        owner.publication::<()>(json!({"kind":"settle","slot":slot,"path":path}))?;
    }
    Ok(())
}

#[cfg(not(verus_keep_ghost))]
pub(crate) struct Publication {
    owner: Inventory,
    path: String,
    open: bool,
}
#[cfg(not(verus_keep_ghost))]
impl Publication {
    fn new(owner: Inventory, path: &str) -> Self {
        Self {
            owner,
            path: path.into(),
            open: true,
        }
    }
    fn member(&self, path: &Path) -> Result<String, String> {
        self.owner
            .relative(path)?
            .strip_prefix(&format!("{}/", self.path))
            .map(str::to_owned)
            .ok_or("member outside publication".into())
    }
    pub(crate) fn check_length(&self, length: u64) -> Result<(), String> {
        self.owner
            .publication(json!({"kind":"check-length","path":self.path,"length":length}))
    }
    pub(crate) fn protect_consumer(&self, consumer: &str) -> Result<Value, String> {
        self.owner
            .publication(json!({"kind":"protect-consumer","path":self.path,"consumer":consumer}))
    }
    pub(crate) fn hard_link_file(&self, source: &Path, dest: &Path) -> Result<(), String> {
        self.owner.publication(
            json!({"kind":"hard-link","path":self.path,"source":source,"name":self.member(dest)?}),
        )
    }

    pub(crate) fn reserve_file(&self, dest: &Path, length: u64) -> Result<PathBuf, String> {
        self.owner.publication(json!({"kind":"reserve-write","path":self.path,"name":self.member(dest)?,"length":length}))
    }
    pub(crate) fn write_file(&self, dest: &Path, bytes: &[u8]) -> Result<(), String> {
        let admitted = self.reserve_file(dest, bytes.len() as u64)?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(admitted)
            .map_err(|e| e.to_string())?;
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|e| e.to_string())
    }
    pub(crate) fn replace_file(&self, target: &Path, bytes: &[u8]) -> Result<(), String> {
        if self.owner.relative(target)? != self.path {
            return Err("replacement outside publication".into());
        }
        let admitted: PathBuf = self
            .owner
            .publication(json!({"kind":"reserve-content","path":self.path,"length":bytes.len()}))?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(admitted)
            .map_err(|e| e.to_string())?;
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|e| e.to_string())
    }
    pub(crate) fn complete(mut self, outcome: Value) -> Result<(), String> {
        self.open = false;
        self.owner
            .publication(json!({"kind":"complete","path":self.path,"outcome":outcome}))
    }
}
#[cfg(not(verus_keep_ghost))]
impl Drop for Publication {
    fn drop(&mut self) {
        if self.open {
            let _ = self
                .owner
                .publication::<()>(json!({"kind":"close","path":self.path}));
        }
    }
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn create_adopted(
    root: &Path,
    producer: &str,
    path: &Path,
    directory: bool,
    outcome: Value,
) -> Result<Option<Publication>, String> {
    if let Some(owner) = Inventory::adopted(root)? {
        let relative = owner.relative(path)?;
        let adopted:bool=owner.publication(json!({"kind":"create-at","path":relative,"producer":producer,"directory":directory,"outcome":outcome}))?;
        return Ok(adopted.then(|| Publication::new(owner, &relative)));
    }
    if directory {
        fs::create_dir(path)
    } else {
        fs::File::create_new(path).map(drop)
    }
    .map_err(|e| e.to_string())?;
    Ok(None)
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn unique_path(directory: &Path, prefix: &str, suffix: &str) -> Result<PathBuf, String> {
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    if prefix.contains('/') || suffix.contains('/') {
        return Err("output address prefix/suffix".into());
    }
    let directory = fs::canonicalize(directory).map_err(|e| e.to_string())?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    Ok(directory.join(format!(
        "{prefix}{}-{now}-{}{suffix}",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    )))
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn copy_adopted(
    p: Option<&Publication>,
    source: &Path,
    dest: &Path,
) -> Result<(), String> {
    if let Some(p) = p {
        p.owner.publication(
            json!({"kind":"copy","path":p.path,"source":source,"name":p.member(dest)?}),
        )
    } else {
        fs::copy(source, dest)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn publish_owned_original(
    root: &Path,
    producer: &str,
    path: &str,
    bytes: &[u8],
    outcome: Value,
) -> Result<bool, String> {
    publish_accepted(
        Inventory::adopted(root)?,
        |owner| owner.accepts(producer, path),
        |owner| {
            owner.session.borrow_mut().publish_bytes(
                json!({"operation":"publish-stream","producer":producer,"path":path,
                "sha256":crate::document::sha256(bytes),"length":bytes.len(),"outcome":outcome}),
                bytes,
            )
        },
    )
}
