//! Enrollment locks protect a worktree and its in-flight Git descendants.
use crate::process_fds::Scope;
use std::{
    fs::{self, File, Metadata, OpenOptions, TryLockError},
    io,
    os::{
        fd::AsFd,
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

fn inode(metadata: &Metadata) -> String {
    format!("{}:{}", metadata.dev(), metadata.ino())
}

pub(crate) fn identity(path: &Path) -> Result<String, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|e| format!("ownership identity {}: {e}", path.display()))?;
    if !metadata.is_file() {
        return Err("ownership lease must be a physical regular file".into());
    }
    Ok(inode(&metadata))
}

pub(crate) struct Lease {
    file: File,
    path: PathBuf,
    id: String,
    directory: bool,
    // Each duplicate shares the kernel open description. The last close releases the lock.
    _scope: Scope,
}

impl Lease {
    fn lock(
        file: File,
        path: &Path,
        expected: Option<&str>,
        directory: bool,
        exclusive: bool,
        wait: Option<Duration>,
    ) -> Result<Option<Self>, String> {
        let metadata = file.metadata().map_err(|e| e.to_string())?;
        if (directory && !metadata.is_dir()) || (!directory && !metadata.is_file()) {
            return Err("unexpected ownership lease type".into());
        }
        let id = inode(&metadata);
        if expected.is_some_and(|expected| expected != id)
            || inode(&fs::symlink_metadata(path).map_err(|e| e.to_string())?) != id
        {
            return Err("ownership lease identity changed; preserve work".into());
        }
        let began = Instant::now();
        loop {
            let result = if exclusive {
                file.try_lock()
            } else {
                file.try_lock_shared()
            };
            match result {
                Ok(()) => break,
                Err(TryLockError::Error(e)) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(TryLockError::Error(e)) => return Err(e.to_string()),
                Err(TryLockError::WouldBlock) => {
                    let Some(bound) = wait else { return Ok(None) };
                    let remaining = bound.saturating_sub(began.elapsed());
                    if remaining.is_zero() {
                        return Err(format!(
                            "ownership remains busy at {}; kernel ownership was not expired",
                            path.display()
                        ));
                    }
                    std::thread::sleep(remaining.min(Duration::from_millis(10)));
                }
            }
        }
        Self::held(file, path, id, directory).map(Some)
    }

    fn held(file: File, path: &Path, id: String, directory: bool) -> Result<Self, String> {
        let scope = Scope::new(&[file.as_fd()])?;
        let lease = Self {
            file,
            path: path.to_owned(),
            id,
            directory,
            _scope: scope,
        };
        lease.stable()?;
        Ok(lease)
    }

    pub(crate) fn acquire(
        path: &Path,
        expected: Option<&str>,
        create: bool,
        exclusive: bool,
        wait: Option<Duration>,
    ) -> Result<Option<Self>, String> {
        let file = Self::open_lease_file(path, expected, create)?;
        Self::lock(file, path, expected, false, exclusive, wait)
    }

    fn open_lease_file(path: &Path, expected: Option<&str>, create: bool) -> Result<File, String> {
        let mut options = OpenOptions::new();
        options
            .read(true)
            .write(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
        let file = if create {
            match options.create_new(true).open(path) {
                Ok(file) => file,
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists && expected.is_none() => options
                    .create_new(false)
                    .open(path)
                    .map_err(|e| e.to_string())?,
                Err(e) => return Err(e.to_string()),
            }
        } else {
            options
                .open(path)
                .map_err(|e| format!("ownership lease unavailable; preserve work: {e}"))?
        };
        Ok(file)
    }

    /// Only an activated worker may block on this physical coordinator's lock.
    pub(crate) fn exclusive_blocking(path: &Path) -> Result<Self, String> {
        let file = Self::open_lease_file(path, None, true)?;
        let metadata = file.metadata().map_err(|e| e.to_string())?;
        if !metadata.is_file() {
            return Err("unexpected ownership lease type".into());
        }
        let id = inode(&metadata);
        file.lock().map_err(|e| e.to_string())?;
        Self::held(file, path, id, false)
    }

    pub(crate) fn stable(&self) -> Result<(), String> {
        let metadata = fs::symlink_metadata(&self.path).map_err(|e| e.to_string())?;
        if metadata.is_dir() != self.directory
            || metadata.is_file() == self.directory
            || inode(&metadata) != self.id
        {
            return Err("held ownership lease identity changed".into());
        }
        Ok(())
    }

    pub(crate) fn capability(&self) -> Result<File, String> {
        self.stable()?;
        self.file.try_clone().map_err(|e| e.to_string())
    }

    pub(crate) fn id(&self) -> &str {
        &self.id
    }
}

pub(crate) struct DirectoryLease(Lease);

impl DirectoryLease {
    pub(crate) fn shared(path: &Path) -> Result<Self, String> {
        Self::blocking(path, false)
    }
    pub(crate) fn blocking(path: &Path, exclusive: bool) -> Result<Self, String> {
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
            .open(path)
            .map_err(|e| e.to_string())?;
        if exclusive {
            file.lock()
        } else {
            file.lock_shared()
        }
        .map_err(|e| e.to_string())?;
        let id = inode(&file.metadata().map_err(|e| e.to_string())?);
        let scope = Scope::new(&[file.as_fd()])?;
        let lease = Self(Lease {
            file,
            path: path.into(),
            id,
            directory: true,
            _scope: scope,
        });
        lease.stable()?;
        Ok(lease)
    }

    pub(crate) fn acquire(
        path: &Path,
        exclusive: bool,
        wait: Option<Duration>,
    ) -> Result<Option<Self>, String> {
        let directory = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
            .open(path)
            .map_err(|e| format!("directory ownership {}: {e}", path.display()))?;
        Lease::lock(directory, path, None, true, exclusive, wait).map(|lease| lease.map(Self))
    }

    pub(crate) fn stable(&self) -> Result<(), String> {
        self.0.stable()
    }
}
