//! Bound processes launched by this worktree. Owned work is joined through native
//! descriptors before an operation is allowed to publish terminal evidence.
use vstd::contrib::exec_spec::ExecSpecEq;
use vstd::prelude::*;

verus! {
pub(crate) fn transport_limit(bytes: usize) -> (valid: bool)
    ensures valid == (1 <= bytes <= 67108864),
{
    bytes > 0 && bytes <= 64 * 1024 * 1024
}

pub(crate) fn executable_matches(expected: Option<&str>, actual: &str) -> (accepted: bool)
    ensures accepted == match expected { None => true, Some(digest) => digest@ == actual@ },
{
    match expected { None => true, Some(digest) => <&str>::exec_eq(digest, actual) }
}
}

#[cfg(not(verus_keep_ghost))]
use crate::document::{digest, file_identity, sha256};
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize};
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    os::{
        fd::AsFd,
        unix::{
            net::UnixStream,
            process::{CommandExt, ExitStatusExt},
        },
    },
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandSpec {
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    pub timeout_seconds: u64,
    pub output_limit_bytes: usize,
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn validate_command(s: &CommandSpec) -> Result<(), String> {
    if s.program.is_empty()
        || s.program.contains('\0')
        || s.args.iter().any(|a| a.contains('\0'))
        || s.env
            .iter()
            .any(|(key, value)| key.is_empty() || key.contains(['=', '\0']) || value.contains('\0'))
        || s.timeout_seconds == 0
        || !transport_limit(s.output_limit_bytes)
    {
        return Err("invalid command or resource bounds".into());
    }
    Ok(())
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn resolve_program(
    root: &Path,
    name: &str,
    environment: Option<&str>,
) -> Result<PathBuf, String> {
    let root = std::path::absolute(root).map_err(|e| e.to_string())?;
    if name.contains('/') {
        let path = root.join(name);
        if path.is_file() {
            return Ok(path);
        }
    } else {
        let path = environment
            .map(OsString::from)
            .or_else(|| std::env::var_os("PATH"))
            .ok_or("PATH missing")?;
        for dir in std::env::split_paths(&path) {
            let candidate = root.join(dir).join(name);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    Err(format!("executable not found: {name}"))
}
#[cfg(not(verus_keep_ghost))]
#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessResult {
    pub(crate) argv: Vec<String>,
    pub(crate) cwd: PathBuf,
    pub(crate) environment: BTreeMap<String, String>,
    pub(crate) environment_digest: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) ownership_fds: Option<String>,
    pub(crate) stdin_sha256: String,
    pub(crate) stdout_bytes: Vec<u8>,
    pub(crate) stderr_bytes: Vec<u8>,
    pub(crate) stdout_complete: bool,
    pub(crate) stderr_complete: bool,
    pub(crate) stdout_sha256: String,
    pub(crate) stderr_sha256: String,
    pub(crate) failure: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) failure_record: Option<Value>,
    pub(crate) exit_code: i32,
    pub(crate) owned_work_joined: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) exit_signal: Option<i32>,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) executable: PathBuf,
    pub(crate) sha256: String,
    pub(crate) launcher_pid: u32,
    pub(crate) child_pid: u32,
    pub(crate) timeout_seconds: u64,
    pub(crate) output_limit_bytes: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) launch_original: Option<Value>,
}
#[cfg(not(verus_keep_ghost))]
pub(crate) struct Evidence<'a> {
    pub(crate) root: &'a Path,
    pub(crate) directory: &'a Path,
    pub(crate) publication: Option<&'a crate::retention::Publication>,
}
#[cfg(not(verus_keep_ghost))]
impl Evidence<'_> {
    fn create(&self, name: &str, limit: u64) -> Result<fs::File, String> {
        let path = self.directory.join(name);
        let path = if let Some(publication) = self.publication {
            publication.reserve_file(&path, limit)?
        } else {
            path
        };
        fs::File::create_new(path).map_err(|e| e.to_string())
    }
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn observed(
    root: &Path,
    s: &CommandSpec,
    input: &[u8],
    expected: Option<&str>,
    stdout_limit: usize,
    evidence: Option<&Evidence<'_>>,
) -> Result<ProcessResult, String> {
    validate_command(s)?;
    if !transport_limit(stdout_limit) {
        return Err("invalid output transport limit".into());
    }
    let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    let executable = resolve_program(&root, &s.program, s.env.get("PATH").map(String::as_str))?;
    let hash = file_identity(&executable)?.0;
    if !executable_matches(expected, &hash) {
        return Err("prelaunch executable digest mismatch".into());
    }
    let mut command = Command::new(&executable);
    if expected.is_some() {
        command.env_clear();
    }
    command
        .current_dir(&root)
        .args(&s.args)
        .envs(&s.env)
        .process_group(0)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut environment = if expected.is_some() {
        BTreeMap::new()
    } else {
        std::env::vars_os()
            .map(|(key, v)| {
                Ok((
                    key.into_string().map_err(|_| "non-UTF8 environment key")?,
                    v.into_string().map_err(|_| "non-UTF8 environment value")?,
                ))
            })
            .collect::<Result<_, String>>()?
    };
    environment.extend(s.env.clone());
    environment.remove("CHRONO_PROCESS_FDS");
    let environment_digest = digest(&environment)?;
    let stdin_sha256 = sha256(input);
    let (lineage, carrier) = UnixStream::pair().map_err(|e| e.to_string())?;
    let ownership_fds = crate::process_fds::forward_command(&mut command, &[carrier.as_fd()])?;
    let argv = std::iter::once(
        executable
            .to_str()
            .ok_or("non-UTF8 executable path")?
            .to_owned(),
    )
    .chain(s.args.clone())
    .collect::<Vec<_>>();
    let launcher_pid = std::process::id();
    let launch_record = |id: u32| {
        json!({"schema":"chrono-process-launch/v2","launcher_pid":launcher_pid,"child_pid":id,
        "argv":argv,"cwd":root,"executable":executable,"sha256":hash,"environment_digest":environment_digest,
        "ownership_fds":ownership_fds,"stdin_sha256":stdin_sha256,"timeout_seconds":s.timeout_seconds,
        "output_limit_bytes":s.output_limit_bytes})
    };
    let mut streams = [None, None];
    let mut launch = None;
    if let Some(original) = evidence {
        streams = [
            Some(original.create("stdout", stdout_limit as u64)?),
            Some(original.create("stderr", s.output_limit_bytes as u64)?),
        ];
        launch = Some(
            original.create(
                "launch.json",
                serde_json::to_vec(&launch_record(u32::MAX))
                    .map_err(|e| e.to_string())?
                    .len() as u64,
            )?,
        );
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("{}: {e}", executable.display()))?;
    let child_pid = child.id();
    drop(command);
    drop(carrier);
    if let Some(mut launch) = launch.take() {
        let stored = serde_json::to_writer(&mut launch, &launch_record(child_pid))
            .map_err(std::io::Error::other)
            .and_then(|()| launch.sync_all());
        if let Err(error) = stored {
            unsafe { libc::kill(-(child_pid as i32), libc::SIGKILL) };
            let _ = child.wait();
            return Err(error.to_string());
        }
    }
    let captured = crate::child_io::capture(
        child,
        lineage,
        input,
        s.timeout_seconds,
        [stdout_limit, s.output_limit_bytes],
        streams,
    )
    .map_err(|e| e.to_string())?;
    let out = captured.stdout.bytes;
    let err = captured.stderr.bytes;
    let failure = captured.failure;
    let failure_record=failure.as_ref().map(|message| json!({"schema":"chrono-error/v1","error":{"type":"chrono_worktree::process","code":"E_PROCESS_BOUND","message":message,"context":{"child_pid":child_pid},"source":null}}));
    let launch_original = if let Some(original) = evidence {
        let path = original.directory.join("launch.json");
        let (digest, _) = file_identity(&path)?;
        let relative = path
            .strip_prefix(original.root)
            .map_err(|e| e.to_string())?;
        Some(json!({"path":relative,"sha256":digest}))
    } else {
        None
    };
    Ok(ProcessResult {
        argv,
        cwd: root,
        environment,
        environment_digest,
        ownership_fds,
        stdin_sha256,
        stdout_complete: captured.stdout.complete,
        stderr_complete: captured.stderr.complete,
        stdout_sha256: sha256(&out),
        stderr_sha256: sha256(&err),
        stdout: String::from_utf8_lossy(&out).into(),
        stderr: String::from_utf8_lossy(&err).into(),
        stdout_bytes: out,
        stderr_bytes: err,
        exit_code: captured.status.code().unwrap_or(-1),
        exit_signal: captured.status.signal(),
        owned_work_joined: captured.joined,
        failure,
        failure_record,
        executable,
        sha256: hash,
        launcher_pid,
        child_pid,
        timeout_seconds: s.timeout_seconds,
        output_limit_bytes: s.output_limit_bytes,
        launch_original,
    })
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn run_process(
    root: &Path,
    s: &CommandSpec,
    input: &[u8],
) -> Result<ProcessResult, String> {
    let process = observed(root, s, input, None, s.output_limit_bytes, None)?;
    if let Some(error) = &process.failure {
        return Err(error.clone());
    }
    Ok(process)
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn run_process_observed(
    root: &Path,
    s: &CommandSpec,
    input: &[u8],
    digest: &str,
) -> Result<ProcessResult, String> {
    observed(root, s, input, Some(digest), s.output_limit_bytes, None)
}
#[cfg(not(verus_keep_ghost))]
pub fn run_process_observed_data(
    root: &Path,
    s: &CommandSpec,
    input: &[u8],
    digest: &str,
    limit: usize,
) -> Result<ProcessResult, String> {
    observed(root, s, input, Some(digest), limit, None)
}
#[cfg(not(verus_keep_ghost))]
#[derive(Serialize)]
pub(crate) struct Tool {
    pub(crate) program: String,
    pub(crate) path: PathBuf,
    pub(crate) basename: String,
    pub(crate) sha256: String,
    pub(crate) version: ProcessResult,
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn tool(
    root: &Path,
    program: &str,
    version: &[String],
    env: &BTreeMap<String, String>,
    timeout: u64,
    limit: usize,
) -> Result<Tool, String> {
    if !program.contains('/') && !env.contains_key("PATH") {
        return Err("tool resolution requires declared PATH".into());
    }
    let path = resolve_program(
        root,
        program,
        Some(env.get("PATH").map(String::as_str).unwrap_or("")),
    )?;
    let hash = file_identity(&path)?.0;
    let s = CommandSpec {
        program: path.to_str().ok_or("non-UTF8 executable")?.into(),
        args: version.to_vec(),
        env: env.clone(),
        timeout_seconds: timeout,
        output_limit_bytes: limit,
    };
    let version = run_process_observed(root, &s, &[], &hash)?;
    Ok(Tool {
        program: program.into(),
        basename: path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("tool basename")?
            .into(),
        path,
        sha256: hash,
        version,
    })
}

#[cfg(not(verus_keep_ghost))]
impl ProcessResult {
    /// Both original streams have already been observed. Encode their checked
    /// bytes once using the public retained-process representation.
    pub fn compact(&self) -> Result<Value, String> {
        let streams = [
            (
                "stdout",
                &self.stdout_bytes,
                &self.stdout,
                &self.stdout_sha256,
            ),
            (
                "stderr",
                &self.stderr_bytes,
                &self.stderr,
                &self.stderr_sha256,
            ),
        ];
        let mut compressed = vec![];
        for (_, bytes, text, hash) in &streams {
            if bytes.len() > 64 * 1024 * 1024
                || String::from_utf8_lossy(bytes) != text.as_str()
                || sha256(bytes) != hash.as_str()
            {
                return Err("inconsistent original process stream".into());
            }
            compressed.push(miniz_oxide::deflate::compress_to_vec_zlib(bytes, 1));
        }
        let zipped = compressed
            .iter()
            .all(|packed| packed.len() <= 64 * 1024 * 1024);
        let mut value = serde_json::to_value(self).map_err(|e| e.to_string())?;
        let record = value.as_object_mut().unwrap();
        for ((name, bytes, _, _), packed) in streams.into_iter().zip(compressed) {
            record.remove(name);
            record.remove(&format!("{name}_bytes"));
            let (label, data) = if zipped {
                record.insert(format!("{name}_length"), json!(bytes.len()));
                (format!("{name}_zlib_hex"), packed.as_slice())
            } else {
                (format!("{name}_hex"), bytes.as_slice())
            };
            let hex = data
                .iter()
                .flat_map(|&byte| [byte >> 4, byte & 15])
                .map(|digit| b"0123456789abcdef"[digit as usize] as char)
                .collect::<String>();
            record.insert(label, json!(hex));
        }
        record.insert(
            "encoding".into(),
            json!(if zipped {
                "chrono-retained-process/v2"
            } else {
                "chrono-retained-process/v1"
            }),
        );
        Ok(value)
    }
    pub(crate) fn validate_launch(&self, path: &Path) -> Result<(), String> {
        let value = crate::document::json(&fs::read(path).map_err(|e| e.to_string())?)?;
        let expected = json!({"schema":"chrono-process-launch/v2","launcher_pid":self.launcher_pid,"child_pid":self.child_pid,"argv":self.argv,"cwd":self.cwd,"executable":self.executable,"sha256":self.sha256,
            "environment_digest":self.environment_digest,"ownership_fds":self.ownership_fds,"stdin_sha256":self.stdin_sha256,"timeout_seconds":self.timeout_seconds,"output_limit_bytes":self.output_limit_bytes});
        if expected != value {
            return Err("launch original differs from joined native process".into());
        }
        Ok(())
    }
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn bind_compiler_directory(args: &mut [String], directory: &str) {
    if args.len() != 12
        || args[..7]
            != [
                "--crate-type=lib",
                "--no-cheating",
                "--no-external-by-default",
                "--num-threads",
                "2",
                "--output-json",
                "--compile",
            ]
        || args[8].is_empty()
        || args[9] != "-o"
        || args[11] != "-Zchecksum-hash-algorithm=sha256"
    {
        return;
    }
    let original = &args[10];
    let Some(name) = original.strip_prefix("{operation_state}/") else {
        return;
    };
    if !name.ends_with(".rlib")
        || name.contains('/')
        || args[7] != format!("--emit=link={original},dep-info={original}.d")
    {
        return;
    }
    let bound = format!("{directory}/{name}");
    args[7] = format!("--emit=link={bound},dep-info={bound}.d");
    args[10] = bound;
}
