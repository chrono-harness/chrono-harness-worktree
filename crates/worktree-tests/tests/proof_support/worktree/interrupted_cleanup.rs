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
