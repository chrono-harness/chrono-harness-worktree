#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
use super::policy;
#[cfg(not(verus_keep_ghost))]
use chrono_worktree::verified_lifecycle as policy;
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

#[cfg(not(verus_keep_ghost))]
use chrono_worktree::process::{CommandSpec, run_process_observed_data};
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use sha2::{Digest, Sha256};
#[cfg(not(verus_keep_ghost))]
use std::{
    collections::BTreeMap,
    path::Path,
    time::{Duration, Instant},
};

#[cfg(not(verus_keep_ghost))]
fn digest(raw: &[u8]) -> String {
    hex::encode(Sha256::digest(raw))
}
#[cfg(not(verus_keep_ghost))]
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
#[cfg(not(verus_keep_ghost))]
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
#[cfg(not(verus_keep_ghost))]
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
#[cfg(not(verus_keep_ghost))]
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

#[test]
#[cfg(not(verus_keep_ghost))]
fn registration_receives_only_the_selected_nested_tool_path() {
    let root = tempfile::tempdir().unwrap();
    let program = env!("CARGO_BIN_EXE_mock-worktree-child");
    let config: chrono_worktree::Config = serde_json::from_value(json!({
        "schema":"chrono-worktree-config/v2", "registration_program":program,
        "host_config":".chrono-harness/config.json", "remote":"origin",
        "git":{"program":"git","expected_version":null,"sha256":null},
        "environment":{"inherit":[],"values":{}}, "timeout_seconds":17,
        "output_limit_bytes":2048, "report_directory":".chrono-harness/state/"
    }))
    .unwrap();
    for path in [None, Some(""), Some("/first install/bin:/second/bin")] {
        let mut host_env = BTreeMap::from([("SSH_AUTH_SOCK".into(), "/host/socket".into())]);
        if let Some(value) = path {
            host_env.insert("PATH".into(), value.into());
        }
        let command = config.registration_command(root.path(), &host_env).unwrap();
        assert_eq!(command.program, program);
        assert_eq!(command.args, ["worktree-inputs-v1"]);
        assert_eq!(
            (command.timeout_seconds, command.output_limit_bytes),
            (17, 2048)
        );
        assert_eq!(command.env.get("PATH").map(String::as_str), path);
        assert_eq!(command.env.len(), usize::from(path.is_some()));
    }
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn maintenance_records_failure_and_still_runs_later_host_operations() {
    let dir = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(
        env!("CARGO_BIN_EXE_mock-worktree-child"),
        dir.path().join("fixture"),
    )
    .unwrap();
    let config = serde_json::from_value(json!({
        "schema":"chrono-worktree-config/v1", "host_config":".chrono-harness/config.json",
        "remote":"origin", "git":{"program":"git"}, "environment":{"inherit":[],"values":{}},
        "timeout_seconds":5, "output_limit_bytes":1024, "report_directory":".chrono-harness/state/worktree/",
        "maintenance_operations":{
            "absent":{"program":"missing","args":[]},
            "failed":{"program":"fixture","args":["streams"]},
            "joined":{"program":"fixture","args":["leave-child", "child-result"]}
        }
    })).unwrap();
    let mut report = json!({});
    let mut failures = Vec::new();
    chrono_worktree::maintain_host_operations(dir.path(), &config, &mut report, &mut failures);
    assert_eq!(failures.len(), 2);
    let result = &report["maintenance_operations"];
    assert!(
        result["absent"]["error"]
            .as_str()
            .unwrap()
            .contains("missing")
    );
    assert_eq!(result["failed"]["exit_code"], 7);
    assert_eq!(result["failed"]["stderr_sha256"], digest(&[255, 254]));
    assert_eq!(result["joined"]["exit_code"], 0);
    assert_eq!(result["joined"]["owned_work_joined"], true);
    assert_eq!(
        std::fs::read(dir.path().join("child-result")).unwrap(),
        b"finished"
    );
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn socket_batches_events_and_retains_new_events_after_owner_failure() {
    use std::os::unix::net::UnixDatagram;
    let (executor, sender) = UnixDatagram::pair().unwrap();
    let mut rounds = 0;
    chrono_worktree::serve_maintenance_socket(&executor, || panic!("empty queue ran a round"))
        .unwrap();
    sender.send(b"first").unwrap();
    sender.send(b"second").unwrap();
    let result = chrono_worktree::serve_maintenance_socket(&executor, || {
        rounds += 1;
        sender.send(b"during-round").unwrap();
        Err("owner failed".into())
    });
    assert_eq!(result.unwrap_err(), "owner failed");
    chrono_worktree::serve_maintenance_socket(&executor, || {
        rounds += 1;
        if rounds == 2 {
            sender.send(b"new-batch").unwrap();
        }
        Ok(())
    })
    .unwrap();
    assert_eq!(rounds, 3);
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn activated_service_preserves_queue_while_competing_for_the_same_lease() {
    use std::{fs, os::unix::net::UnixDatagram, sync::mpsc, thread};
    // Unix socket names must fit sockaddr_un on both supported kernels.
    let dir = tempfile::tempdir_in("/tmp").unwrap();
    let root = fs::canonicalize(dir.path()).unwrap();
    let state = root.join(".chrono-harness/state");
    fs::create_dir_all(&state).unwrap();
    fs::write(root.join(".chrono-harness/maintenance.json"), serde_json::to_vec(&json!({
        "schema":"chrono-unified-maintenance/v2","state_directory":".chrono-harness/state/owner/",
        "socket":".chrono-harness/state/socket"
    })).unwrap()).unwrap();
    let socket = UnixDatagram::bind(state.join("socket")).unwrap();
    let service = || socket.try_clone().unwrap();
    let result =
        chrono_worktree::run_maintenance_service_socket(&root, "missing", service()).unwrap();
    assert_eq!(result["status"], "maintenance-queue-empty");
    assert!(!state.join("owner").exists());

    fs::create_dir(state.join("owner")).unwrap();
    let lease = fs::File::create(state.join("owner/maintenance.lease")).unwrap();
    lease.lock().unwrap();
    UnixDatagram::unbound()
        .unwrap()
        .send_to(b"work", state.join("socket"))
        .unwrap();
    let (arrived, before_lease) = mpsc::channel();
    let (resume, finish) = mpsc::channel();
    let worker_root = root.clone();
    let inherited = service();
    let worker = thread::spawn(move || {
        chrono_worktree::with_maintenance_observer(
            move |kind, _| {
                if kind == "service-before-lease" {
                    arrived.send(()).unwrap();
                    finish.recv().unwrap();
                }
            },
            || {
                chrono_worktree::run_maintenance_service_socket(
                    &worker_root,
                    ".chrono-harness/missing",
                    inherited,
                )
            },
        )
    });
    before_lease.recv_timeout(Duration::from_secs(10)).unwrap();
    resume.send(()).unwrap();
    // The lock owner's successor can still take the queued hint; a crashed
    // waiter must not consume it before ownership is obtained.
    socket.set_nonblocking(true).unwrap();
    let mut original = [0u8; 4];
    assert_eq!(socket.recv(&mut original).unwrap(), 4);
    assert_eq!(&original, b"work");
    drop(lease);
    assert_eq!(
        worker.join().unwrap().unwrap()["status"],
        "maintenance-queue-empty"
    );

    UnixDatagram::unbound()
        .unwrap()
        .send_to(b"next", state.join("socket"))
        .unwrap();
    let error = chrono_worktree::run_maintenance_service_socket(
        &root,
        ".chrono-harness/missing",
        service(),
    )
    .unwrap_err();
    assert!(error.contains("No such file"), "{error}");
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn socket_starts_a_second_round_for_the_sixty_fifth_queued_hint() {
    use std::os::unix::net::UnixDatagram;
    let (executor, sender) = UnixDatagram::pair().unwrap();
    rustix::net::sockopt::set_socket_recv_buffer_size(&executor, 262_144).unwrap();
    sender.set_nonblocking(true).unwrap();
    for _ in 0..65 {
        sender.send(b"x").unwrap();
    }
    let mut rounds = 0;
    chrono_worktree::serve_maintenance_socket(&executor, || {
        rounds += 1;
        Ok(())
    })
    .unwrap();
    assert_eq!(rounds, 2);
}

verus! {
fn expect(value: bool) requires value { if !value { panic!(); } }

#[cfg_attr(not(verus_keep_ghost), test)]
fn maintenance_selects_a_round_only_for_received_events() {
    expect(!policy::maintenance_has_events(0));
    expect(policy::maintenance_has_events(1));
    expect(policy::maintenance_has_events(64));
}
}

#[cfg(verus_keep_ghost)]
verus! {
fn kernel_wait_boundaries(nanos: u128, interrupted: bool) {
    use policy::child_io::{poll_millis, poll_reply, PollReply};
    let delay = poll_millis(nanos);
    assert((delay == 0) == (nanos == 0));
    if nanos > 0 && nanos <= 2147483647000000u128 {
        assert(delay as int == (nanos as int - 1) / 1000000 + 1);
        assert((delay as int - 1) * 1000000 < nanos && nanos <= delay as int * 1000000);
    }
    if nanos > 2147483647000000u128 { assert(delay == i32::MAX); }
    let one = poll_millis(1);
    let exact = poll_millis(1000000);
    let partial = poll_millis(1000001);
    assert(one == 1 && exact == 1 && partial == 2);
    let zero = poll_reply(0, interrupted);
    let error = poll_reply(-1, false);
    let signal = poll_reply(-1, true);
    let event = poll_reply(1, interrupted);
    assert(zero is Again && error is Failed && signal is Again && event is Ready);
}
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn registered_path_reply_belongs_to_the_exact_query() {
    use chrono_worktree::registered_nonartifact_paths;
    let paths = ["说明.txt", "cache/a", "message.py", "说明.txt"].map(String::from);
    let artifacts = serde_json::json!([{"path":"cache/","tracked":false}]);
    let outside = "registration returned paths outside the queried batch";
    for (reply, accepted) in [
        (vec![], true),
        (vec!["说明.txt", "message.py", "说明.txt"], true),
        (vec!["说明.txt", "说明.txt"], true),
        (vec!["说明.txt", "说明.txt", "说明.txt"], false),
        (vec!["message.py", "cache/a"], false),
        (vec!["unqueried.txt"], false),
    ] {
        let reply: Vec<String> = reply.into_iter().map(String::from).collect();
        let result = registered_nonartifact_paths(&artifacts, &paths, |request| {
            assert_eq!(
                *request,
                serde_json::json!({
                    "operation":"nonartifact-paths", "config":{"artifacts":artifacts}, "paths":paths,
                })
            );
            Ok(reply.clone())
        });
        assert_eq!(
            result,
            if accepted {
                Ok(reply)
            } else {
                Err(outside.into())
            }
        );
    }
    assert_eq!(
        registered_nonartifact_paths(&artifacts, &paths, |_| Err("query failed".into())),
        Err("query failed".into()),
    );
}
