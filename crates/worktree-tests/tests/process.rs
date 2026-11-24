use chrono_worktree::process::{CommandSpec, run_process_observed_data};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::Path,
    time::{Duration, Instant},
};

fn digest(raw: &[u8]) -> String {
    format!("{:x}", Sha256::digest(raw))
}
fn run(
    root: &Path,
    argv: Vec<String>,
    input: &[u8],
    out: usize,
    err: usize,
    timeout: u64,
    drift: bool,
) -> Result<(Value, Value), String> {
    let program = env!("CARGO_BIN_EXE_mock-worktree-child");
    let hash = if drift {
        "0".repeat(64)
    } else {
        digest(&std::fs::read(program).unwrap())
    };
    let cmd = CommandSpec {
        program: program.into(),
        args: argv,
        env: BTreeMap::from([("SAMPLE".into(), "declared".into())]),
        timeout_seconds: timeout,
        output_limit_bytes: err,
    };
    let receipt = run_process_observed_data(root, &cmd, input, &hash, out)?;
    Ok((serde_json::to_value(&receipt).unwrap(), receipt.compact()?))
}
#[test]
fn bounded_process_records_raw_streams_actual_pid_and_checked_compaction() {
    let dir = tempfile::tempdir().unwrap();
    let input = [0, 1, 0xff, 0, 10];
    let (p, compact) = run(
        dir.path(),
        vec!["streams".into()],
        &input,
        input.len(),
        2,
        5,
        false,
    )
    .unwrap();
    assert!(p["failure"].is_null());
    assert_eq!(p["stdout_bytes"], json!(input));
    assert_eq!(p["stderr_bytes"], json!([255, 254]));
    assert_eq!(p["exit_code"], 7);
    assert_eq!(p["owned_work_joined"], true);
    assert_eq!(p["stdout_complete"], true);
    assert_eq!(p["stderr_complete"], true);
    assert_eq!(p["launcher_pid"], std::process::id());
    assert_ne!(p["child_pid"], p["launcher_pid"]);
    assert!(p["child_pid"].as_u64().unwrap() > 0);
    assert_eq!(p["stdin_sha256"], digest(&input));
    assert_eq!(p["environment"], json!({"SAMPLE":"declared"}));
    assert!(compact.get("stdout_bytes").is_none());
    assert_eq!(compact["stdout_length"], input.len());
    assert_eq!(compact["encoding"], "chrono-retained-process/v2");
    assert!(run(dir.path(), vec!["streams".into()], &input, 8, 2, 5, true).is_err());
}
#[test]
fn timeout_and_output_limit_leave_partial_results_and_reap_the_child() {
    let dir = tempfile::tempdir().unwrap();
    let (p, _) = run(dir.path(), vec!["burst".into()], b"", 8, 8, 4, false).unwrap();
    assert_eq!(p["stdout_bytes"], json!([171; 8].to_vec()));
    assert_eq!(p["stdout_complete"], false);
    assert!(p["failure"].as_str().unwrap().contains("bound"));
    let t = Instant::now();
    let (p, _) = run(dir.path(), vec!["wait".into()], b"", 10, 10, 1, false).unwrap();
    assert!(p["failure"].as_str().unwrap().contains("timeout"));
    assert!(p["exit_signal"].as_i64().is_some());
    assert!(t.elapsed() < Duration::from_secs(8));
    assert_eq!(p["owned_work_joined"], true);
}
#[test]
fn child_reaping_alone_never_marks_inherited_native_work_joined() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("child-did-finish");
    let (p, _) = run(
        dir.path(),
        vec!["leave-child".into(), marker.to_str().unwrap().into()],
        b"",
        10,
        10,
        5,
        false,
    )
    .unwrap();
    assert_eq!(p["exit_code"], 0);
    assert!(p["failure"].is_null());
    assert_eq!(p["owned_work_joined"], true);
    assert_eq!(std::fs::read(marker).unwrap(), b"finished");
}
