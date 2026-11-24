use chrono_worktree::CliOutput;
use serde_json::{Value, json};
use std::{
    ffi::OsString,
    io::{self, BufRead, BufReader, Write},
    os::fd::AsFd,
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, ExitCode, Stdio},
    sync::{Arc, Mutex},
};

fn line(writer: &mut impl Write, value: &Value) -> io::Result<()> {
    serde_json::to_writer(&mut *writer, value)?;
    writer.write_all(b"\n")?;
    writer.flush()
}
struct Diagnostics {
    child: Child,
    writer: Option<ChildStdin>,
}
impl Drop for Diagnostics {
    fn drop(&mut self) {
        self.writer.take();
        let _ = self.child.wait();
    }
}
impl Diagnostics {
    fn open(root: &Path, operation: &str) -> io::Result<Option<Self>> {
        match std::fs::symlink_metadata(root.join(".chrono-harness/logging.json")) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
            Ok(_) => (),
        }
        let mut child = Command::new(chrono_worktree::installed_tool("chrono-diagnostics"))
            .args(["cli-events-v1"])
            .arg(root)
            .args(["chrono-worktree", operation])
            .arg(std::process::id().to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        let input = child.stdin.take();
        let output = child.stdout.take().unwrap();
        let session = Self {
            child,
            writer: input,
        };
        let mut reply = String::new();
        BufReader::new(output).read_line(&mut reply)?;
        if serde_json::from_str::<Value>(&reply)? != json!({"ready":true}) {
            return Err(io::Error::other("unexpected cli-events-v1 ready reply"));
        }
        Ok(Some(session))
    }
    fn execute(&mut self, args: &[String]) -> io::Result<u8> {
        let state = Arc::new(Mutex::new((
            self.writer.take().unwrap(),
            Ok::<(), io::Error>(()),
        )));
        let sink = Arc::clone(&state);
        let sender = tracing_tunnel::TracingEventSender::new(move |event| {
            let mut state = sink.lock().unwrap();
            let (writer, result) = &mut *state;
            if result.is_ok() {
                *result = line(writer, &json!({"event":event}));
            }
        });
        let code = tracing::subscriber::with_default(sender, || execute(args));
        let (writer, written) = Arc::try_unwrap(state)
            .map_err(|_| io::Error::other("live worktree diagnostic writer"))?
            .into_inner()
            .unwrap();
        self.writer = Some(writer);
        written?;
        code
    }
    fn finish(mut self, code: u8) -> io::Result<u8> {
        line(self.writer.as_mut().unwrap(), &json!({"exit_code":code}))?;
        self.writer.take();
        let status = self.child.wait()?;
        match status.code() {
            Some(actual) if actual == i32::from(code) => Ok(code),
            Some(2) => Ok(2),
            _ => Err(io::Error::other(format!(
                "cli-events-v1 returned {status} after worktree exit {code}"
            ))),
        }
    }
}
fn write_bytes(destination: impl AsFd, mut bytes: &[u8]) -> io::Result<()> {
    while !bytes.is_empty() {
        match rustix::io::write(&destination, bytes) {
            Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
            Ok(n) => bytes = &bytes[n..],
            Err(rustix::io::Errno::INTR) => continue,
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}
fn execute(args: &[String]) -> io::Result<u8> {
    let CliOutput {
        exit_code,
        stdout,
        stderr,
    } = chrono_worktree::run(args);
    if !stderr.is_empty() {
        tracing::error!(event_id="cli.diagnostic",message=%stderr);
    }
    let first = write_bytes(io::stdout().lock(), stdout.as_bytes());
    let second = write_bytes(io::stderr().lock(), stderr.as_bytes());
    first.and(second)?;
    Ok(exit_code)
}
fn run(args: Vec<OsString>) -> io::Result<u8> {
    let root = args
        .iter()
        .skip(1)
        .zip(args.iter().skip(2))
        .find(|(key, _)| *key == "--host-root")
        .map(|(_, value)| Ok(PathBuf::from(value)))
        .unwrap_or_else(std::env::current_dir)?;
    let operation = args.get(1).and_then(|arg| arg.to_str()).unwrap_or("");
    let mut session = Diagnostics::open(&root, operation)?;
    let mut decoded = Vec::new();
    for (index, arg) in args.iter().enumerate() {
        let Some(text) = arg.to_str() else {
            let raw = arg.as_encoded_bytes();
            if let Some(mut sink) = session {
                line(
                    sink.writer.as_mut().unwrap(),
                    &json!({"invalid_argument":{"argv_index":index,"bytes":raw}}),
                )?;
                return sink.finish(2);
            }
            let error = json!({"schema":"chrono-error/v1","error":{"type":"chrono_worktree::cli","code":"E_USAGE","message":"arguments must be valid UTF-8.","context":{"argv_index":index,"argument_index":index.checked_sub(1),"argument_bytes":raw},"source":null}});
            write_bytes(io::stderr(), format!("{error}\n").as_bytes())?;
            return Ok(2);
        };
        if index != 0 {
            decoded.push(text.to_owned());
        }
    }
    let Some(mut sink) = session.take() else {
        return execute(&decoded);
    };
    let code = sink.execute(&decoded)?;
    sink.finish(code)
}
fn main() -> ExitCode {
    let code = run(std::env::args_os().collect()).unwrap_or_else(|error| {
        let _ = write_bytes(
            io::stderr(),
            format!("chrono-worktree: {error}\n").as_bytes(),
        );
        2
    });
    ExitCode::from(code)
}
