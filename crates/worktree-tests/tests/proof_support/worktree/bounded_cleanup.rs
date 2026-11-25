#[cfg(verus_keep_ghost)]
use super::{expect, policy};

#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {
#[cfg_attr(not(verus_keep_ghost), test)]
fn use_intent_uses_one_completed_operation_and_its_own_process() {
    proof {
        reveal_strlit(""); reveal_strlit("/checkout"); reveal_strlit("/other");
        reveal_strlit("validate.delta"); reveal_strlit("other");
        reveal_strlit("check.json"); reveal_strlit("another.json");
        reveal_strlit("check.json.process"); reveal_strlit("another.process");
        reveal_strlit("chrono-worktree-managed-use-intent/v2");
        reveal_strlit("chrono-worktree-managed-use-result/v1");
        assert("/checkout"@.len() != ""@.len() && "validate.delta"@.len() != ""@.len());
        assert("/checkout"@.len() != "/other"@.len() && "validate.delta"@.len() != "other"@.len());
        assert("check.json"@.len() != "another.json"@.len());
        assert("check.json.process"@.len() != "another.process"@.len());

    }
    let intent = policy::ManagedUseBinding {
        schema: "chrono-worktree-managed-use-intent/v2", checkout: "/checkout",
        operation: "validate.delta", report_path: "check.json", process_path: "check.json.process",
        joined: false,
    };
    let result = policy::ManagedUseBinding {
        schema: "chrono-worktree-managed-use-result/v1", joined: true, ..intent
    };
    expect(policy::binds_use_intent(&intent, &result, "check.json", "check.json.process"));
    expect(!policy::binds_use_intent(&result, &intent, "check.json", "check.json.process"));
    let incomplete = policy::ManagedUseBinding { joined: false, ..result };
    expect(!policy::binds_use_intent(&intent, &incomplete, "check.json", "check.json.process"));
    let other = policy::ManagedUseBinding { checkout: "/other", ..result };
    expect(!policy::binds_use_intent(&intent, &other, "check.json", "check.json.process"));
    let different = policy::ManagedUseBinding { operation: "other", ..result };
    expect(!policy::binds_use_intent(&intent, &different, "check.json", "check.json.process"));
    let empty = policy::ManagedUseBinding { operation: "", ..intent };
    let missing = policy::ManagedUseBinding { operation: "", ..result };
    expect(!policy::binds_use_intent(&empty, &missing, "check.json", "check.json.process"));
    expect(!policy::binds_use_intent(&intent, &result, "another.json", "check.json.process"));
    expect(!policy::binds_use_intent(&intent, &result, "check.json", "another.process"));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn terminal_history_preserves_unfinished_responsibilities_and_unknown_ownership() {
    expect(policy::terminal_history_ready(0, false, false, false, Some(false)));
    expect(!policy::terminal_history_ready(1, false, false, false, Some(false)));
    expect(!policy::terminal_history_ready(0, true, false, false, Some(false)));
    expect(!policy::terminal_history_ready(0, false, true, false, Some(false)));
    expect(!policy::terminal_history_ready(0, false, false, true, Some(false)));
    expect(!policy::terminal_history_ready(0, false, false, false, Some(true)));
    expect(!policy::terminal_history_ready(0, false, false, false, None));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn terminal_history_checks_each_supported_evidence_source() {
    expect(policy::terminal_history_handoff(Some(true), false, false));
    expect(policy::terminal_history_handoff(Some(false), true, false));
    expect(!policy::terminal_history_handoff(Some(false), false, true));
    expect(policy::terminal_history_handoff(None, true, true));
    expect(!policy::terminal_history_handoff(None, true, false));
    expect(!policy::terminal_history_handoff(None, false, true));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn terminal_history_requires_bound_disposal_under_both_originals() {
    broadcast use vstd::string::group_string_axioms;
    proof {
        reveal_strlit("cleaned"); assert("cleaned"@.len() == 7);
        reveal_strlit("failed"); assert("failed"@.len() == 6);
        reveal_strlit("preserved"); assert("preserved"@.len() == 9);
        reveal_strlit("verified-absent"); assert("verified-absent"@.len() == 15);
        reveal_strlit("already-absent"); assert("already-absent"@.len() == 14);
    }
    expect(policy::terminal_history_result(true, true, true, true, true, "cleaned", "verified-absent"));
    expect(policy::terminal_history_result(true, true, true, true, true, "cleaned", "already-absent"));
    expect(!policy::terminal_history_result(true, true, true, true, true, "failed", "verified-absent"));
    expect(!policy::terminal_history_result(true, true, true, true, true, "cleaned", "preserved"));
    expect(!policy::terminal_history_result(false, true, true, true, true, "cleaned", "verified-absent"));
    expect(!policy::terminal_history_result(true, false, true, true, true, "cleaned", "verified-absent"));
    expect(!policy::terminal_history_result(true, true, false, true, true, "cleaned", "verified-absent"));
    expect(!policy::terminal_history_result(true, true, true, false, true, "cleaned", "verified-absent"));
    expect(!policy::terminal_history_result(true, true, true, true, false, "cleaned", "verified-absent"));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn cache_history_requires_both_the_exact_intent_and_the_completed_result() {
    broadcast use vstd::string::group_string_axioms;
    proof { reveal_strlit("cleaned"); assert("cleaned"@.len() == 7);
        reveal_strlit("failed"); assert("failed"@.len() == 6); }
    let exact = policy::CacheHistoryBinding { receipt_matches: true, result_checkout_matches: true,
        result_generation_matches: true, result_intent_matches: true, intent_checkout_matches: true,
        intent_attachment_matches: true, intent_generation_matches: true, owner_matches: true };
    expect(policy::cache_history_result(&exact,"cleaned"));
    expect(!policy::cache_history_result(&exact,"failed"));
    let bad = policy::CacheHistoryBinding { receipt_matches: false, ..exact };
    expect(!policy::cache_history_result(&bad, "cleaned"));
    let bad = policy::CacheHistoryBinding { result_checkout_matches: false, ..exact };
    expect(!policy::cache_history_result(&bad, "cleaned"));
    let bad = policy::CacheHistoryBinding { result_generation_matches: false, ..exact };
    expect(!policy::cache_history_result(&bad, "cleaned"));
    let bad = policy::CacheHistoryBinding { result_intent_matches: false, ..exact };
    expect(!policy::cache_history_result(&bad, "cleaned"));
    let bad = policy::CacheHistoryBinding { intent_checkout_matches: false, ..exact };
    expect(!policy::cache_history_result(&bad, "cleaned"));
    let bad = policy::CacheHistoryBinding { intent_attachment_matches: false, ..exact };
    expect(!policy::cache_history_result(&bad, "cleaned"));
    let bad = policy::CacheHistoryBinding { intent_generation_matches: false, ..exact };
    expect(!policy::cache_history_result(&bad, "cleaned"));
    let bad = policy::CacheHistoryBinding { owner_matches: false, ..exact };
    expect(!policy::cache_history_result(&bad, "cleaned"));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn terminal_history_keeps_unknown_future_live_and_unexpired_records() {
    let boundary = policy::retire_history(true, true, Some(100), 110, 10, false);
    let young = policy::retire_history(true, true, Some(100), 109, 10, false);
    let capacity = policy::retire_history(true, true, Some(100), 100, 10, true);
    let unknown = policy::retire_history(true, true, None, 110, 0, true);
    let future = policy::retire_history(true, true, Some(111), 110, 0, true);
    let active = policy::retire_history(false, true, Some(100), 110, 0, true);
    let busy = policy::retire_history(true, false, Some(100), 110, 0, true);
    let maximum = policy::retire_history(true, true, Some(u64::MAX), u64::MAX, 1, false);
    expect(boundary && !young && capacity && !unknown && !future && !active && !busy && !maximum);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn main_enrollment_report_is_released_when_its_last_handoff_ends() {
    expect(!policy::needs_enrollment_report(true, false, false));
    expect(policy::needs_enrollment_report(true, true, false));
    expect(policy::needs_enrollment_report(true, false, true));
    expect(policy::needs_enrollment_report(false, false, false));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn cache_history_releases_a_settled_main_generation_and_keeps_pending_results() {
    let expired = policy::retire_cache_history(true, 1, 2, false, true, Some(100), 110, 10, false);
    let capacity = policy::retire_cache_history(true, 1, 2, false, true, Some(100), 100, 10, true);
    let current = policy::retire_cache_history(true, 2, 2, false, true, Some(100), 110, 0, true);
    let future = policy::retire_cache_history(true, 3, 2, false, true, Some(100), 110, 0, true);
    let unknown = policy::retire_cache_history(true, 1, 2, false, true, None, 110, 0, true);
    let incomplete = policy::retire_cache_history(false, 1, 2, false, true, Some(100), 110, 0, true);
    let busy = policy::retire_cache_history(true, 1, 2, false, false, Some(100), 110, 0, true);
    let settled = policy::retire_cache_history(true, 2, 2, true, true, Some(100), 110, 10, false);
    let before = policy::retire_cache_history(true, 2, 2, true, true, Some(100), 109, 10, false);
    let unfinished = policy::retire_cache_history(false, 2, 2, true, true, Some(100), 110, 10, false);
    let held = policy::retire_cache_history(true, 2, 2, true, false, Some(100), 110, 10, false);
    expect(settled && !before && !unfinished && !held);
    expect(expired && capacity && !current && !future && !unknown && !incomplete && !busy);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn reusable_outputs_wait_for_release_and_evidence_keeps_its_own_permission() {
    use policy::Disposition::*;
    expect(policy::select_artifact(&Dispose, false, false));
    expect(policy::select_artifact(&Dispose, true, true));
    expect(!policy::select_artifact(&FinishDispose, false, false));
    expect(!policy::select_artifact(&FinishDispose, false, true));
    expect(policy::select_artifact(&FinishDispose, true, false));
    expect(policy::select_artifact(&FinishDispose, true, true));
    expect(!policy::select_artifact(&Retain, false, false));
    expect(!policy::select_artifact(&Retain, true, true));
    expect(!policy::select_artifact(&EvidenceRetain, true, false));
    expect(policy::select_artifact(&EvidenceRetain, false, true));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn release_requests_bind_only_the_requested_cache_generation() {
    expect(!policy::reusable_release(false, false, 7, None));
    expect(policy::reusable_release(false, false, 7, Some(7)));
    expect(!policy::reusable_release(false, false, 8, Some(7)));
    expect(!policy::reusable_release(false, false, 7, Some(8)));
    expect(!policy::reusable_release(false, false, u64::MAX, None));
    expect(!policy::reusable_release(false, false, u64::MAX, Some(0)));
    expect(policy::reusable_release(false, false, u64::MAX, Some(u64::MAX)));
}
}
