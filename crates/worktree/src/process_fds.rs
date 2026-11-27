//! Kernels keep a file-description lease held until its last native descendant closes it.
use std::collections::BTreeSet;
use vstd::prelude::*;

verus! {
pub(crate) fn insert_descriptor(seen: &mut BTreeSet<i32>, fd: i32) -> (accepted: bool)
    ensures
        accepted == (fd >= 3 && !old(seen)@.contains(fd)),
        seen@ == if fd >= 3 { old(seen)@.insert(fd) } else { old(seen)@ },
{
    fd >= 3 && seen.insert(fd)
}

/// The first duplicate starts at 202. Leave the shell's descriptor 255 available.
pub(crate) fn reserved_copy(fd: i32) -> (reserved: bool)
    ensures reserved == (fd == 255),
{ fd == 255 }
}

#[cfg(not(verus_keep_ghost))]
use std::{
    cell::RefCell,
    io,
    marker::PhantomData,
    os::{
        fd::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd, RawFd},
        unix::process::CommandExt,
    },
    process::Command,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};
#[cfg(not(verus_keep_ghost))]
const ENV: &str = "CHRONO_PROCESS_FDS";
#[cfg(not(verus_keep_ghost))]
thread_local! {static SCOPES:RefCell<Vec<(u64, Vec<OwnedFd>)>>=const{RefCell::new(Vec::new())};}
#[cfg(not(verus_keep_ghost))]
static NEXT: AtomicU64 = AtomicU64::new(1);
#[cfg(not(verus_keep_ghost))]
fn owned_copy(fd: RawFd) -> io::Result<OwnedFd> {
    if fd < 3 {
        return Err(io::Error::other("ownership descriptor aliases stdio"));
    }
    let duplicate = |min: libc::c_int| {
        let result = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, min) };
        if result < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(unsafe { OwnedFd::from_raw_fd(result) })
    };
    let copy = duplicate(202)?;
    if reserved_copy(copy.as_raw_fd()) {
        duplicate(256)
    } else {
        Ok(copy)
    }
}
#[cfg(not(verus_keep_ghost))]
pub(crate) struct Scope {
    id: u64,
    _thread: PhantomData<Rc<()>>,
}
#[cfg(not(verus_keep_ghost))]
impl Scope {
    pub(crate) fn new(fds: &[BorrowedFd<'_>]) -> Result<Self, String> {
        let copies = fds
            .iter()
            .map(|fd| owned_copy(fd.as_raw_fd()))
            .collect::<io::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        SCOPES.with(|scopes| scopes.borrow_mut().push((id, copies)));
        Ok(Self {
            id,
            _thread: PhantomData,
        })
    }
}
#[cfg(not(verus_keep_ghost))]
impl Drop for Scope {
    fn drop(&mut self) {
        SCOPES.with(|s| s.borrow_mut().retain(|(id, _)| *id != self.id))
    }
}
#[cfg(not(verus_keep_ghost))]
fn inherited() -> Result<Vec<RawFd>, String> {
    let Some(value) = std::env::var_os(ENV) else {
        return Ok(vec![]);
    };
    let text = value.to_str().ok_or("ownership list is not UTF-8")?;
    let mut seen = BTreeSet::new();
    text.split(',')
        .map(|item| {
            let fd: RawFd = item.parse().map_err(|_| "invalid ownership descriptor")?;
            if !insert_descriptor(&mut seen, fd) {
                return Err("invalid or duplicate ownership descriptor".into());
            }
            Ok(fd)
        })
        .collect()
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn forward_command(
    command: &mut Command,
    extra: &[BorrowedFd<'_>],
) -> Result<Option<String>, String> {
    if command.get_envs().any(|(key, _)| key == ENV) {
        return Err("ownership descriptors already bound".into());
    }
    let old = inherited()?;
    let mut copies = old
        .iter()
        .copied()
        .map(owned_copy)
        .collect::<io::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;
    SCOPES
        .with(|scopes| -> io::Result<()> {
            for fd in scopes.borrow().iter().flat_map(|(_, fds)| fds.iter()) {
                copies.push(owned_copy(fd.as_raw_fd())?);
            }
            Ok(())
        })
        .map_err(|e| e.to_string())?;
    for fd in extra {
        copies.push(owned_copy(fd.as_raw_fd()).map_err(|e| e.to_string())?);
    }
    if copies.is_empty() {
        command.env_remove(ENV);
        return Ok(None);
    }
    let carrier = copies
        .iter()
        .map(|fd| fd.as_raw_fd().to_string())
        .collect::<Vec<_>>()
        .join(",");
    command.env(ENV, &carrier);
    // The parent retains CLOEXEC copies in Command. Only modify their child-side flags.
    unsafe {
        command.pre_exec(move || {
            for fd in &old {
                if libc::fcntl(*fd, libc::F_SETFD, libc::FD_CLOEXEC) < 0 {
                    return Err(io::Error::last_os_error());
                }
            }
            for fd in &copies {
                if libc::fcntl(fd.as_raw_fd(), libc::F_SETFD, 0) < 0 {
                    return Err(io::Error::last_os_error());
                }
            }
            Ok(())
        });
    }
    Ok(Some(carrier))
}
