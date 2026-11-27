//! 提醒分类的独立边界预期；Cargo 和 Verus 消费同一生产实现。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../worktree/src/verified_attention.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_worktree::verified_attention as policy;
#[cfg(not(verus_keep_ghost))]
use vstd::prelude::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn snapshot_protocol_requires_the_matching_admission_contract() {
    let v1 = policy::snapshot_protocol("chrono-worktree-enrollment-state/v1", false);
    let v2 = policy::snapshot_protocol("chrono-worktree-enrollment-state/v2", true);
    let extra = policy::snapshot_protocol("chrono-worktree-enrollment-state/v1", true);
    let missing = policy::snapshot_protocol("chrono-worktree-enrollment-state/v2", false);
    let unknown = policy::snapshot_protocol("unknown", true);
    proof {
        reveal_strlit("chrono-worktree-enrollment-state/v1");
        reveal_strlit("chrono-worktree-enrollment-state/v2");
        reveal_strlit("unknown");
        assert("chrono-worktree-enrollment-state/v1"@[34] != "chrono-worktree-enrollment-state/v2"@[34]);
        assert("unknown"@.len() != "chrono-worktree-enrollment-state/v2"@.len());
    }
    assert(v1 is Unleased);
    assert(v2 is Kernel);
    assert(extra is Invalid);
    assert(missing is Invalid);
    assert(unknown is Invalid);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn snapshot_owner_and_original_bytes_must_all_match() {
    let valid = policy::snapshot_owned(&policy::SnapshotProtocol::Kernel, true, true, true, true, true);
    let marker = policy::snapshot_owned(&policy::SnapshotProtocol::Kernel, false, true, true, true, true);
    let coordinator = policy::snapshot_owned(&policy::SnapshotProtocol::Kernel, true, false, true, true, true);
    let common = policy::snapshot_owned(&policy::SnapshotProtocol::Kernel, true, true, false, true, true);
    let address = policy::snapshot_owned(&policy::SnapshotProtocol::Kernel, true, true, true, false, true);
    let admission = policy::snapshot_owned(&policy::SnapshotProtocol::Kernel, true, true, true, true, false);
    let invalid = policy::snapshot_owned(&policy::SnapshotProtocol::Invalid, true, true, true, true, true);
    let unleased = policy::snapshot_owned(&policy::SnapshotProtocol::Unleased, true, true, true, true, false);
    assert(valid && unleased && !marker && !coordinator && !common && !address && !admission && !invalid);
    let stable = policy::snapshot_stable(true, true, true);
    let moved = policy::snapshot_stable(false, true, true);
    let changed_owner = policy::snapshot_stable(true, false, true);
    let changed_ledger = policy::snapshot_stable(true, true, false);
    assert(stable && !moved && !changed_owner && !changed_ledger);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn active_and_retained_work_still_need_attention() {
    let active = policy::classify("active", false, true, true, true, true, true, false);
    let retained = policy::classify("retained", false, true, true, true, true, true, true);
    let pending = policy::classify("terminal", false, true, true, true, true, true, true);
    proof {
        reveal_strlit("active"); reveal_strlit("retained");
        reveal_strlit("terminal"); reveal_strlit("disposed");
        assert("terminal"@[0] != "retained"@[0]);
    }
    assert(active is Active);
    assert(retained is Retained);
    assert(pending is TerminalPending);
    let a = policy::needs_attention(&active);
    let r = policy::needs_attention(&retained);
    let p = policy::needs_attention(&pending);
    assert(a && r && p);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn removal_requires_both_absences_and_a_terminal_handoff() {
    let removed = policy::classify("disposed", false, true, true, false, false, false, true);
    let physical = policy::classify("disposed", false, true, true, true, false, false, true);
    let git = policy::classify("disposed", false, true, true, false, true, false, true);
    let unfinished = policy::classify("disposed", false, true, true, false, false, false, false);
    assert(removed is Disposed);
    assert(physical is Unknown);
    assert(git is Unknown);
    assert(unfinished is Unknown);
    let quiet = policy::needs_attention(&removed);
    let alert = policy::needs_attention(&physical);
    assert(!quiet && alert);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn unknown_unregistered_and_coordinator_are_distinct() {
    let unknown = policy::classify("active", false, false, true, true, true, true, false);
    let unattached = policy::classify("active", false, true, true, true, true, false, false);
    let unregistered = policy::classify("", false, true, false, true, true, false, false);
    let main = policy::classify("active", true, true, true, true, true, true, false);
    assert(unknown is Unknown);
    assert(unattached is Unknown);
    assert(unregistered is Unregistered);
    assert(main is Coordinator);
    let a = policy::needs_attention(&unknown);
    let b = policy::needs_attention(&unregistered);
    let c = policy::needs_attention(&main);
    assert(a && b && !c);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn pending_agent_reminders_require_the_same_nonempty_creator_and_unfinished_cleanup() {
    broadcast use vstd::string::group_string_axioms;
    proof {
        reveal_strlit("agent-A"); reveal_strlit("agent-B"); reveal_strlit("");
        assert("agent-A"@.len() == 7 && "agent-B"@.len() == 7 && ""@.len() == 0);
        assert("agent-A"@[6] != "agent-B"@[6]);
    }
    let own = policy::pending_for_agent("agent-A", "agent-A", true);
    let other = policy::pending_for_agent("agent-B", "agent-A", true);
    let disposed = policy::pending_for_agent("agent-A", "agent-A", false);
    let absent = policy::pending_for_agent("", "", true);
    assert(own && !other && !disposed && !absent);
}


}

#[cfg(not(verus_keep_ghost))]
#[test]
fn closing_the_maintenance_notice_reader_preserves_the_handoff() {
    use std::{env, fs, io, path::Path, process::Command};

    const CHILD_ROOT: &str = "CHRONO_TEST_MAINTENANCE_NOTICE_ROOT";
    if let Some(root) = env::var_os(CHILD_ROOT) {
        chrono_worktree::announce_maintenance(Path::new(&root), "published-handoff");
        return;
    }

    let host = tempfile::tempdir().unwrap();
    let config = host.path().join(".chrono-harness");
    fs::create_dir_all(config.join("state")).unwrap();
    fs::write(
        config.join("maintenance.json"),
        br#"{"schema":"chrono-unified-maintenance/v2","state_directory":".chrono-harness/state/automatic-cleanup-v2/","socket":".chrono-harness/state/maintenance.sock"}"#,
    )
    .unwrap();
    let mut child = Command::new(env::current_exe().unwrap());
    child
        .args([
            "--exact",
            "closing_the_maintenance_notice_reader_preserves_the_handoff",
            "--nocapture",
        ])
        .env_remove("CHRONO_MAINTENANCE_CONTEXT")
        .env(CHILD_ROOT, host.path());
    let visible = child.output().unwrap();
    assert!(visible.status.success());
    assert!(
        String::from_utf8(visible.stderr)
            .unwrap()
            .contains("wake-failed")
    );
    let (reader, writer) = io::pipe().unwrap();
    drop(reader);
    let closed = child.stderr(writer).output().unwrap();
    assert!(closed.status.success(), "child returned {}", closed.status);
}
