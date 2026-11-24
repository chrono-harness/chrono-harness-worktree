//! Versioned final-original protocol over a Unix socket. JSON travels on stdout and kernel capabilities on the socket.
use rustix::net::{RecvAncillaryBuffer, RecvAncillaryMessage, RecvFlags, ReturnFlags, recvmsg};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, IoSliceMut, Write},
    mem::MaybeUninit,
    net::Shutdown,
    os::{
        fd::{AsFd, OwnedFd},
        unix::net::UnixStream,
    },
    path::Path,
    process::{Child, ChildStdout, Command, Stdio},
};

pub(crate) struct Session {
    child: Child,
    socket: UnixStream,
    output: Option<BufReader<ChildStdout>>,
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.socket.shutdown(Shutdown::Write);
        drop(self.output.take());
        let _ = self.child.wait();
    }
}

impl Session {
    pub(crate) fn open(root: &Path) -> Result<Self, String> {
        let (socket, input) = UnixStream::pair().map_err(|e| e.to_string())?;
        let program = crate::installed_tool("chrono-harness");
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
            output: Some(output),
        })
    }

    fn send(&mut self, request: Value) -> Result<(), String> {
        serde_json::to_writer(&mut self.socket, &request).map_err(|e| e.to_string())?;
        self.socket.write_all(b"\n").map_err(|e| e.to_string())
    }

    fn receive(&mut self) -> Result<String, String> {
        let mut line = String::new();
        if self
            .output
            .as_mut()
            .expect("session output")
            .read_line(&mut line)
            .map_err(|e| e.to_string())?
            == 0
        {
            let status = self.child.wait().map_err(|e| e.to_string())?;
            return Err(format!(
                "retained-artifacts-v1 ended without a response: {status}"
            ));
        }
        Ok(line)
    }

    pub(crate) fn publish_bytes(&mut self, request: Value, bytes: &[u8]) -> Result<bool, String> {
        self.send(request)?;
        let ready: Value =
            serde_json::from_str(&self.receive()?).map_err(|error| error.to_string())?;
        if ready != json!({"receive": bytes.len()}) {
            return Err(format!(
                "original owner did not accept the content stream: {ready}"
            ));
        }
        self.socket
            .write_all(bytes)
            .map_err(|error| error.to_string())?;
        serde_json::from_str::<Result<bool, String>>(&self.receive()?)
            .map_err(|error| error.to_string())?
    }

    pub(crate) fn call<T: DeserializeOwned>(&mut self, request: Value) -> Result<T, String> {
        self.send(request)?;
        serde_json::from_str::<Result<T, String>>(&self.receive()?).map_err(|e| e.to_string())?
    }

    /// The owner emits observations as they occur. The inventory writer consumes each row immediately.
    pub(crate) fn observe_maintenance(
        &mut self,
        mut observe: impl FnMut(Value),
    ) -> Result<Value, String> {
        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum Frame {
            Observation { observation: Value },
            Complete(Result<Value, String>),
        }
        self.send(json!({"operation":"observe-maintenance"}))?;
        loop {
            let frame =
                serde_json::from_str::<Frame>(&self.receive()?).map_err(|e| e.to_string())?;
            match frame {
                Frame::Observation { observation } => observe(observation),
                Frame::Complete(outcome) => return outcome,
            }
        }
    }

    pub(crate) fn descriptors(&mut self) -> Result<Vec<OwnedFd>, String> {
        const BATCH: usize = 32;
        let mut received = Vec::new();
        loop {
            let count: usize =
                self.call(json!({"operation":"scoped-descriptors","offset":received.len()}))?;
            if count > BATCH {
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
