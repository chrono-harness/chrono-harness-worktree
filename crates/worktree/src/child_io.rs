//! Wait on original stdio, child exit and the inherited open-description sentinel.
//! waitid(WNOWAIT) pins the child PID until group cancellation and final reap finish.
use std::{
    io::{self, Read, Write},
    os::{
        fd::{AsRawFd, RawFd},
        unix::net::UnixStream,
    },
    process::{Child, ExitStatus},
    thread,
    time::{Duration, Instant},
};

fn nonblocking(fd: RawFd) -> io::Result<()> {
    let options = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if options < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, options | libc::O_NONBLOCK) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
fn observe_exit(pid: u32) -> io::Result<()> {
    loop {
        let mut info = std::mem::MaybeUninit::<libc::siginfo_t>::zeroed();
        let status = unsafe {
            libc::waitid(
                libc::P_PID,
                pid as libc::id_t,
                info.as_mut_ptr(),
                libc::WEXITED | libc::WNOWAIT,
            )
        };
        if status == 0 {
            return Ok(());
        }
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error);
        }
    }
}
struct OwnedChild(Option<Child>);
impl OwnedChild {
    fn cancel(&self) {
        if let Some(child) = &self.0 {
            unsafe { libc::kill(-(child.id() as i32), libc::SIGKILL) };
        }
    }
    fn reap(mut self) -> io::Result<ExitStatus> {
        let status = self.0.as_mut().unwrap().wait()?;
        self.0.take();
        Ok(status)
    }
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        self.cancel();
        if let Some(child) = &mut self.0 {
            let _ = child.wait();
        }
    }
}
pub(crate) struct Stream {
    pub(crate) bytes: Vec<u8>,
    pub(crate) complete: bool,
    limit: usize,
    exceeded: bool,
}
impl Stream {
    fn new(limit: usize) -> Self {
        Self {
            bytes: vec![],
            complete: false,
            limit,
            exceeded: false,
        }
    }
    fn receive(
        &mut self,
        input: &mut impl Read,
        original: &mut Option<std::fs::File>,
    ) -> io::Result<bool> {
        let mut buffer = [0; 65536];
        match input.read(&mut buffer) {
            Ok(0) => {
                self.complete = !self.exceeded;
                Ok(true)
            }
            Ok(n) => {
                let count = n.min(self.limit - self.bytes.len());
                if let Some(file) = original {
                    file.write_all(&buffer[..count])?;
                }
                self.bytes.extend_from_slice(&buffer[..count]);
                self.exceeded |= count != n;
                Ok(false)
            }
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock
                ) =>
            {
                Ok(false)
            }
            Err(e) => Err(e),
        }
    }
}

pub(crate) struct Captured {
    pub(crate) status: ExitStatus,
    pub(crate) stdout: Stream,
    pub(crate) stderr: Stream,
    pub(crate) joined: bool,
    pub(crate) failure: Option<String>,
}
fn descriptor(fd: Option<RawFd>, events: libc::c_short) -> libc::pollfd {
    libc::pollfd {
        fd: fd.unwrap_or(-1),
        events,
        revents: 0,
    }
}
fn wait(descriptors: &mut [libc::pollfd], deadline: Instant) -> io::Result<bool> {
    let millis = deadline
        .saturating_duration_since(Instant::now())
        .as_millis();
    let timeout = millis.min(i32::MAX as u128) as i32;
    let result = unsafe { libc::poll(descriptors.as_mut_ptr(), descriptors.len() as _, timeout) };
    if result < 0 {
        let e = io::Error::last_os_error();
        if e.kind() != io::ErrorKind::Interrupted {
            return Err(e);
        }
    }
    Ok(result > 0)
}
pub(crate) fn capture(
    child: Child,
    lineage: UnixStream,
    mut input: &[u8],
    timeout: u64,
    limits: [usize; 2],
    mut originals: [Option<std::fs::File>; 2],
) -> io::Result<Captured> {
    let mut child = OwnedChild(Some(child));
    let direct = child.0.as_mut().unwrap();
    let mut stdin = direct.stdin.take();
    let mut stdout = direct.stdout.take();
    let mut stderr = direct.stderr.take();
    if input.is_empty() {
        stdin.take();
    }
    let mut lineage = Some(lineage);
    let (reader, notify) = UnixStream::pair()?;
    let mut terminal = Some(reader);
    let mut deadline = Instant::now()
        .checked_add(Duration::from_secs(timeout))
        .ok_or_else(|| io::Error::other("timeout cannot be represented"))?;
    let mut captured_out = Stream::new(limits[0]);
    let mut captured_err = Stream::new(limits[1]);
    let mut failure = None;
    for fd in [
        stdin.as_ref().map(AsRawFd::as_raw_fd),
        stdout.as_ref().map(AsRawFd::as_raw_fd),
        stderr.as_ref().map(AsRawFd::as_raw_fd),
        lineage.as_ref().map(AsRawFd::as_raw_fd),
    ]
    .into_iter()
    .flatten()
    {
        nonblocking(fd)?;
    }
    let pid = direct.id();
    let notified = thread::Builder::new()
        .name("worktree-child-exit".into())
        .spawn(move || {
            let result = observe_exit(pid);
            drop(notify);
            result
        })?;
    let mut cancelled = false;
    let result = (|| -> io::Result<()> {
        while stdin.is_some()
            || stdout.is_some()
            || stderr.is_some()
            || lineage.is_some()
            || terminal.is_some()
        {
            if failure.is_some() && !cancelled {
                child.cancel();
                cancelled = true;
                // Reap the direct child even if a detached descendant retains an original.
                deadline = Instant::now() + Duration::from_secs(2);
                stdin.take();
            }
            let mut fds = [
                descriptor(stdin.as_ref().map(AsRawFd::as_raw_fd), libc::POLLOUT),
                descriptor(stdout.as_ref().map(AsRawFd::as_raw_fd), libc::POLLIN),
                descriptor(stderr.as_ref().map(AsRawFd::as_raw_fd), libc::POLLIN),
                descriptor(lineage.as_ref().map(AsRawFd::as_raw_fd), libc::POLLIN),
                descriptor(terminal.as_ref().map(AsRawFd::as_raw_fd), libc::POLLIN),
            ];
            if Instant::now() >= deadline
                || !wait(&mut fds, deadline)? && Instant::now() >= deadline
            {
                if cancelled {
                    break;
                }
                failure.get_or_insert_with(|| "worktree command exceeded its timeout".into());
                continue;
            }
            if fds[0].revents != 0 {
                let writer = stdin.as_mut().unwrap();
                match writer.write(input) {
                    Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
                    Ok(n) => input = &input[n..],
                    Err(e)
                        if matches!(
                            e.kind(),
                            io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                        ) => {}
                    Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {
                        stdin.take();
                    }
                    Err(e) => return Err(e),
                }
                if input.is_empty() {
                    stdin.take();
                }
            }
            if fds[1].revents != 0
                && captured_out.receive(stdout.as_mut().unwrap(), &mut originals[0])?
            {
                stdout.take();
            }
            if fds[2].revents != 0
                && captured_err.receive(stderr.as_mut().unwrap(), &mut originals[1])?
            {
                stderr.take();
            }
            if fds[3].revents != 0 {
                let mut buffer = [0; 256];
                match lineage.as_mut().unwrap().read(&mut buffer) {
                    Ok(0) => {
                        lineage.take();
                    }
                    Ok(_) => {
                        failure.get_or_insert_with(|| {
                            "unexpected bytes on ownership descriptor".into()
                        });
                    }
                    Err(e)
                        if matches!(
                            e.kind(),
                            io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                        ) => {}
                    Err(e) => return Err(e),
                }
            }
            if fds[4].revents != 0 {
                terminal.take();
            }
            if captured_out.exceeded || captured_err.exceeded {
                failure.get_or_insert_with(|| "worktree command exceeded its stream bound".into());
            }
        }
        for original in originals.into_iter().flatten() {
            original.sync_all()?;
        }
        Ok(())
    })();
    if result.is_err() {
        child.cancel();
    }
    let observed = notified
        .join()
        .map_err(|_| io::Error::other("child exit observation thread panicked"))?;
    let status = child.reap()?;
    result?;
    observed?;
    Ok(Captured {
        status,
        stdout: captured_out,
        stderr: captured_err,
        joined: lineage.is_none(),
        failure,
    })
}
