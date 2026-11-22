#[cfg(verus_keep_ghost)]
use vstd::contrib::exec_spec::ExecSpecEq;
#[cfg(verus_keep_ghost)]
use super::{expect, policy};

#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {
#[cfg_attr(not(verus_keep_ghost), test)]
fn imported_policy_requires_published_original_and_exact_receipt_fields() {
    broadcast use vstd::string::group_string_axioms;
    proof {
        reveal_strlit("chrono-worktree-report/v1"); reveal_strlit("chrono-worktree-report/v2");
        reveal_strlit("import"); reveal_strlit("migrate");
        reveal_strlit("imported"); reveal_strlit("failed");
        reveal_strlit("/owner"); reveal_strlit("/owner\0");
        reveal_strlit("report"); reveal_strlit("report/other");
        reveal_strlit("config"); reveal_strlit("config\0");
        assert("chrono-worktree-report/v1"@[24] != "chrono-worktree-report/v2"@[24]);
        assert("import"@.len() == 6 && "migrate"@.len() == 7);
        assert("imported"@.len() == 8 && "failed"@.len() == 6);
        assert("/owner"@.len() == 6 && "/owner\0"@.len() == 7);
        assert("report"@.len() == 6 && "report/other"@.len() == 12);
        assert("config"@.len() == 6 && "config\0"@.len() == 7);
    }
    let mut r = policy::ImportedPolicyReceipt {
        schema: Some("chrono-worktree-report/v1"), operation: Some("import"),
        status: Some("imported"), source_root: Some("/owner"),
        report_path: Some("report"), config_path: Some("config"),
    };
    expect(policy::imported_policy_receipt_valid(&r, true, "/owner", "report", "config"));
    expect(!policy::imported_policy_receipt_valid(&r, false, "/owner", "report", "config"));
    r.schema = None;
    expect(!policy::imported_policy_receipt_valid(&r, true, "/owner", "report", "config"));
    r.schema = Some("chrono-worktree-report/v2");
    expect(!policy::imported_policy_receipt_valid(&r, true, "/owner", "report", "config"));
    r.schema = Some("chrono-worktree-report/v1");
    r.operation = Some("migrate");
    expect(!policy::imported_policy_receipt_valid(&r, true, "/owner", "report", "config"));
    r.operation = Some("import"); r.status = Some("failed");
    expect(!policy::imported_policy_receipt_valid(&r, true, "/owner", "report", "config"));
    r.status = Some("imported"); r.source_root = Some("/owner\0");
    expect(!policy::imported_policy_receipt_valid(&r, true, "/owner", "report", "config"));
    r.source_root = Some("/owner"); r.report_path = Some("report/other");
    expect(!policy::imported_policy_receipt_valid(&r, true, "/owner", "report", "config"));
    r.report_path = Some("report"); r.config_path = Some("config\0");
    expect(!policy::imported_policy_receipt_valid(&r, true, "/owner", "report", "config"));
    r.config_path = None;
    expect(!policy::imported_policy_receipt_valid(&r, true, "/owner", "report", "config"));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_adoption_requires_active_joined_nonterminal_work() {
    broadcast use vstd::string::group_string_axioms;
    proof {
        reveal_strlit("active"); reveal_strlit("terminal");
        reveal_strlit("retained"); reveal_strlit("Active");
        reveal_strlit("active\0"); reveal_strlit("");
        assert("active"@[0] != "Active"@[0]);
        assert("active"@.len() == 6 && "active\0"@.len() == 7 && ""@.len() == 0);
    }
    expect(policy::checkout_migration_ready("active", false, false, false));
    expect(!policy::checkout_migration_ready("active", true, false, false));
    expect(!policy::checkout_migration_ready("active", false, true, false));
    expect(!policy::checkout_migration_ready("active", false, false, true));
    expect(!policy::checkout_migration_ready("active", true, true, true));
    expect(!policy::checkout_migration_ready("terminal", false, false, false));
    expect(!policy::checkout_migration_ready("retained", false, false, false));
    expect(!policy::checkout_migration_ready("Active", false, false, false));
    expect(!policy::checkout_migration_ready("active\0", false, false, false));
    expect(!policy::checkout_migration_ready("", false, false, false));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn current_policy_handoff_keeps_one_binding_and_no_original_chain() {
    expect(!policy::policy_state_present(false, false));
    expect(policy::policy_state_present(false, true));
    expect(policy::policy_state_present(true, false));
    expect(policy::policy_state_present(true, true));
    expect(!policy::policy_state_conflict(false, false));
    expect(!policy::policy_state_conflict(false, true));
    expect(!policy::policy_state_conflict(true, false));
    expect(policy::policy_state_conflict(true, true));
    let empty = policy::policy_migration_decision(false, false, false, false);
    expect(!empty.store_current && !empty.proceed);
    let legacy = policy::policy_migration_decision(false, true, false, false);
    expect(legacy.store_current && legacy.proceed);
    let current = policy::policy_migration_decision(true, false, false, false);
    expect(!current.store_current && !current.proceed);
    let checkout = policy::policy_migration_decision(true, false, false, true);
    expect(!checkout.store_current && checkout.proceed);
    let changed = policy::policy_migration_decision(true, false, true, false);
    expect(changed.store_current && changed.proceed);
    let both = policy::policy_migration_decision(false, true, true, true);
    expect(both.store_current && both.proceed);

    let mut binding = None;
    let mut originals = Vec::new();
    originals.push(7u64);
    originals.push(9u64);
    policy::store_current_policy(&mut binding, &mut originals, 13u64);
    expect(binding == Some(13u64) && originals.len() == 0);
    policy::store_current_policy(&mut binding, &mut originals, 17u64);
    expect(binding == Some(17u64) && originals.len() == 0);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn current_policy_binding_requires_exact_inputs_and_preserves_error_priority() {
    expect(policy::adopted_policy_binding_error(true, true, true, true, true, true).is_none());
    let digest = policy::adopted_policy_binding_error(false, true, true, true, true, true);
    expect(<&str>::exec_eq(digest.unwrap(), "invalid current adopted policy binding"));
    let keys = policy::adopted_policy_binding_error(true, false, true, true, true, true);
    expect(<&str>::exec_eq(keys.unwrap(), "invalid current adopted policy binding"));
    let inputs = policy::adopted_policy_binding_error(true, true, false, true, true, true);
    expect(<&str>::exec_eq(inputs.unwrap(), "invalid current adopted policy binding"));
    let source_keys = policy::adopted_policy_binding_error(true, true, true, false, true, true);
    expect(<&str>::exec_eq(source_keys.unwrap(), "adopted cleanup source does not bind policy inputs"));
    let source_digest = policy::adopted_policy_binding_error(true, true, true, true, false, true);
    expect(<&str>::exec_eq(source_digest.unwrap(), "adopted cleanup source does not bind policy inputs"));
    let mismatch = policy::adopted_policy_binding_error(true, true, true, true, true, false);
    expect(<&str>::exec_eq(mismatch.unwrap(), "current adopted policy digest differs from its committed inputs"));
    let all_invalid = policy::adopted_policy_binding_error(false, false, false, false, false, false);
    expect(<&str>::exec_eq(all_invalid.unwrap(), "invalid current adopted policy binding"));
    let invalid_source = policy::adopted_policy_binding_error(true, true, true, false, false, false);
    expect(<&str>::exec_eq(invalid_source.unwrap(), "adopted cleanup source does not bind policy inputs"));
}
}
