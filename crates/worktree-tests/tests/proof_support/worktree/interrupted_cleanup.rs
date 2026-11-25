#[cfg(verus_keep_ghost)]
use super::expect_stage;

#[cfg(verus_keep_ghost)]
use super::{expect, policy};

#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {
#[cfg_attr(not(verus_keep_ghost), test)]
fn published_and_preserved_receipts_keep_their_distinct_authorities() {
    broadcast use vstd::string::group_string_axioms;
    proof {
        reveal_strlit("cleaned"); assert("cleaned"@.len() == 7);
        reveal_strlit("failed"); assert("failed"@.len() == 6);
        reveal_strlit("unknown-status"); assert("unknown-status"@.len() == 14);
        reveal_strlit("preserved"); assert("preserved"@.len() == 9);
        reveal_strlit("verified-absent"); assert("verified-absent"@.len() == 15);
        reveal_strlit("disposed"); assert("disposed"@.len() == 8);
        reveal_strlit("active"); assert("active"@.len() == 6);
    }
    let valid = policy::AutomaticCleanupReceipt { schema_matches: true, cleanup_operation: true, automatic: true, host_matches: true, config_matches: true, attempt_matches: true, token_matches: true };
    expect(policy::published_cleanup_receipt(&valid,"cleaned", Some(true), false));
    expect(policy::published_cleanup_receipt(&valid,"failed", None, true));
    expect(!policy::published_cleanup_receipt(&valid,"cleaned", Some(false), true));
    expect(!policy::published_cleanup_receipt(&valid,"failed", None, false));
    expect(!policy::published_cleanup_receipt(&valid,"unknown-status", Some(true), true));
    expect(policy::artifacts_only_terminal("disposed", true, false));
    expect(!policy::artifacts_only_terminal("active", true, false));
    expect(!policy::artifacts_only_terminal("disposed", false, false));
    expect(!policy::artifacts_only_terminal("disposed", true, true));
    expect(policy::preserved_cleanup_consumed_terminal(&valid,true,true,true,"cleaned","preserved",true,true));
    expect(!policy::preserved_cleanup_consumed_terminal(&valid,true,true,true,"failed","preserved",true,true));
    expect(!policy::preserved_cleanup_consumed_terminal(&valid,true,true,true,"cleaned","verified-absent",true,true));
    let wrong = policy::AutomaticCleanupReceipt { schema_matches: false, ..valid };
    expect(!policy::automatic_receipt_matches(&wrong));
    let wrong = policy::AutomaticCleanupReceipt { cleanup_operation: false, ..valid };
    expect(!policy::automatic_receipt_matches(&wrong));
    let wrong = policy::AutomaticCleanupReceipt { automatic: false, ..valid };
    expect(!policy::automatic_receipt_matches(&wrong));
    let wrong = policy::AutomaticCleanupReceipt { host_matches: false, ..valid };
    expect(!policy::automatic_receipt_matches(&wrong));
    let wrong = policy::AutomaticCleanupReceipt { config_matches: false, ..valid };
    expect(!policy::automatic_receipt_matches(&wrong));
    let wrong = policy::AutomaticCleanupReceipt { attempt_matches: false, ..valid };
    expect(!policy::automatic_receipt_matches(&wrong));
    let wrong = policy::AutomaticCleanupReceipt { token_matches: false, ..valid };
    expect(!policy::automatic_receipt_matches(&wrong));
    expect(!policy::preserved_cleanup_consumed_terminal(&valid,false,true,true,"cleaned","preserved",true,true));
    expect(!policy::preserved_cleanup_consumed_terminal(&valid,true,false,true,"cleaned","preserved",true,true));
    expect(!policy::preserved_cleanup_consumed_terminal(&valid,true,true,false,"cleaned","preserved",true,true));
    expect(!policy::preserved_cleanup_consumed_terminal(&valid,true,true,true,"cleaned","preserved",false,true));
    expect(!policy::preserved_cleanup_consumed_terminal(&valid,true,true,true,"cleaned","preserved",true,false));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn interrupted_cleanup_requires_each_original_identity() {
    broadcast use vstd::string::group_string_axioms;
    proof {
        reveal_strlit("cleanup"); assert("cleanup"@.len() == 7);
        reveal_strlit("other-op"); assert("other-op"@.len() == 8);
        reveal_strlit("different"); assert("different"@.len() == 9);
        reveal_strlit("token"); assert("token"@.len() == 5);
        reveal_strlit("path"); assert("path"@.len() == 4);
        reveal_strlit("checkout"); assert("checkout"@.len() == 8);
        reveal_strlit("branch"); assert("branch"@.len() == 6);
        reveal_strlit("head"); assert("head"@.len() == 4);
        reveal_strlit("registry"); assert("registry"@.len() == 8);
    }
    let expected = policy::CleanupBinding { token:"token", report_path:"path", checkout:"checkout",
        branch:"branch", head:"head", registry_digest:"registry" };
    expect(policy::binds_interrupted_cleanup("cleanup", &expected, &expected));
    expect(!policy::binds_interrupted_cleanup("other-op", &expected, &expected));
    let changed = policy::CleanupBinding { token:"different", ..expected };
    expect(!policy::binds_interrupted_cleanup("cleanup", &changed, &expected));
    let changed = policy::CleanupBinding { report_path:"different", ..expected };
    expect(!policy::binds_interrupted_cleanup("cleanup", &changed, &expected));
    let changed = policy::CleanupBinding { checkout:"different", ..expected };
    expect(!policy::binds_interrupted_cleanup("cleanup", &changed, &expected));
    let changed = policy::CleanupBinding { branch:"different", ..expected };
    expect(!policy::binds_interrupted_cleanup("cleanup", &changed, &expected));
    let changed = policy::CleanupBinding { head:"different", ..expected };
    expect(!policy::binds_interrupted_cleanup("cleanup", &changed, &expected));
    let changed = policy::CleanupBinding { registry_digest:"different", ..expected };
    expect(!policy::binds_interrupted_cleanup("cleanup", &changed, &expected));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn only_a_successful_original_with_absence_verification_establishes_disposal() {
    broadcast use vstd::string::group_string_axioms;
    proof {
        reveal_strlit("cleaned"); reveal_strlit("failed");
        reveal_strlit("verified-absent"); reveal_strlit("already-absent");
        reveal_strlit("preserved"); reveal_strlit("");
        assert("cleaned"@.len() == 7 && "failed"@.len() == 6);
        assert("preserved"@.len() == 9 && "verified-absent"@.len() == 15
            && "already-absent"@.len() == 14 && ""@.len() == 0);
    }
    let removed = policy::cleanup_result(false, "cleaned", "verified-absent");
    let recovered = policy::cleanup_result(false, "cleaned", "already-absent");
    let failed = policy::cleanup_result(false, "failed", "verified-absent");
    let retained = policy::cleanup_result(false, "cleaned", "preserved");
    let missing = policy::cleanup_result(false, "cleaned", "");
    let cache = policy::cleanup_result(true, "cleaned", "verified-absent");
    expect_stage(removed, policy::CleanupStage::Disposed);
    expect_stage(recovered, policy::CleanupStage::Disposed);
    expect_stage(failed, policy::CleanupStage::Failed);
    expect_stage(retained, policy::CleanupStage::Retained);
    expect_stage(missing, policy::CleanupStage::Retained);
    expect_stage(cache, policy::CleanupStage::CachesReleased);
}
}
