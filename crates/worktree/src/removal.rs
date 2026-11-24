//! The final-original owner excludes overlapping publications while this worktree removes outputs.
use rustix::net::{RecvAncillaryBuffer, RecvAncillaryMessage, RecvFlags, ReturnFlags, recvmsg};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    fs,
    io::{BufRead, BufReader, IoSliceMut, Write},
    mem::MaybeUninit,
    net::Shutdown,
    os::{
        fd::{AsFd, OwnedFd},
        unix::net::UnixStream,
    },
    path::{Path, PathBuf},
    process::{Child, ChildStdout, Command, Stdio},
};

pub(crate) struct Guard {
    session: RefCell<Session>,
    // The subprocess and any deliberately handed-off child share the original open descriptions.
    _capabilities: chrono_harness::process_fds::Scope,
}

impl Guard {
    pub(crate) fn one(root: &Path, path: &str) -> Result<Option<Self>, String> {
        Self::open(root, &[path.to_owned()])
    }

    pub(crate) fn open(root: &Path, paths: &[String]) -> Result<Option<Self>, String> {
        for path in paths {
            chrono_harness::relative_path(path)?;
        }
        match fs::symlink_metadata(root.join(".chrono-harness/retention.json")) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.to_string()),
            Ok(_) => {}
        }
        let mut session = Session::open(root)?;
        if !session.call::<bool>(json!({"operation":"open-removal","paths":paths}))? {
            return Ok(None);
        }
        let fds = session.descriptors()?;
        let borrowed = fds.iter().map(AsFd::as_fd).collect::<Vec<_>>();
        let capabilities = chrono_harness::process_fds::Scope::new(&borrowed)?;
        Ok(Some(Self {
            session: RefCell::new(session),
            _capabilities: capabilities,
        }))
    }

    pub(crate) fn stable(&self) -> Result<(), String> {
        self.session
            .borrow_mut()
            .call(json!({"operation":"check-removal"}))
    }
}

struct Session {
    child: Child,
    socket: UnixStream,
    output: BufReader<ChildStdout>,
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.socket.shutdown(Shutdown::Write);
        let _ = self.child.wait();
    }
}

impl Session {
    fn open(root: &Path) -> Result<Self, String> {
        let (socket, input) = UnixStream::pair().map_err(|e| e.to_string())?;
        // The installed user timer invokes the host's binary link with a system-only PATH.
        let name = Path::new("chrono-harness");
        let on_path = std::env::var_os("PATH")
            .is_some_and(|paths| std::env::split_paths(&paths).any(|dir| dir.join(name).is_file()));
        let program = if on_path {
            name.to_owned()
        } else {
            std::env::args_os()
                .next()
                .map(PathBuf::from)
                .and_then(|arg| {
                    arg.parent()
                        .filter(|dir| !dir.as_os_str().is_empty())
                        .map(Path::to_owned)
                })
                .and_then(|dir| std::path::absolute(dir).ok())
                .map_or_else(|| name.to_owned(), |dir| dir.join(name))
        };
        let mut child = Command::new(program)
            .arg("retained-artifacts-v1")
            .current_dir(root)
            .stdin(Stdio::from(OwnedFd::from(input)))
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| format!("retained-artifacts-v1: {e}"))?;
        let output = BufReader::new(child.stdout.take().expect("piped runner output"));
        Ok(Self {
            child,
            socket,
            output,
        })
    }

    fn call<T: DeserializeOwned>(&mut self, request: Value) -> Result<T, String> {
        serde_json::to_writer(&mut self.socket, &request).map_err(|e| e.to_string())?;
        self.socket.write_all(b"\n").map_err(|e| e.to_string())?;
        let mut line = String::new();
        if self
            .output
            .read_line(&mut line)
            .map_err(|e| e.to_string())?
            == 0
        {
            let status = self.child.wait().map_err(|e| e.to_string())?;
            return Err(format!(
                "retained-artifacts-v1 ended without a response: {status}"
            ));
        }
        serde_json::from_str::<Result<T, String>>(&line).map_err(|e| e.to_string())?
    }

    fn descriptors(&mut self) -> Result<Vec<OwnedFd>, String> {
        const BATCH: usize = 32;
        let mut received = Vec::new();
        loop {
            let count: usize =
                self.call(json!({"operation":"scoped-descriptors","offset":received.len()}))?;
            if count > BATCH || (count == 0 && received.is_empty()) {
                return Err("invalid removal descriptor count".into());
            }
            if count == 0 {
                return Ok(received);
            }
            let mut marker = [0u8];
            let mut buffer = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(BATCH))];
            let mut controls = RecvAncillaryBuffer::new(&mut buffer);
            #[cfg(target_os = "linux")]
            let flags = RecvFlags::CMSG_CLOEXEC;
            #[cfg(not(target_os = "linux"))]
            let flags = RecvFlags::empty();
            let message = recvmsg(
                &self.socket,
                &mut [IoSliceMut::new(&mut marker)],
                &mut controls,
                flags,
            )
            .map_err(|e| e.to_string())?;
            if message.bytes != 1 || marker != [1] || message.flags.contains(ReturnFlags::CTRUNC) {
                return Err("invalid removal descriptor transfer".into());
            }
            let before = received.len();
            for message in controls.drain() {
                if let RecvAncillaryMessage::ScmRights(fds) = message {
                    for fd in fds {
                        rustix::io::fcntl_setfd(fd.as_fd(), rustix::io::FdFlags::CLOEXEC)
                            .map_err(|e| e.to_string())?;
                        received.push(fd);
                    }
                }
            }
            if received.len() - before != count {
                return Err("incomplete removal descriptor transfer".into());
            }
            if count < BATCH {
                return Ok(received);
            }
        }
    }
}
