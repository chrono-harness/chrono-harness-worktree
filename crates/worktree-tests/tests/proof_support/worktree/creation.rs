#[cfg(verus_keep_ghost)]
use super::{expect, policy};

#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn cli_success_requires_an_original_success_status_and_includes_service_handoffs() {
    broadcast use vstd::string::group_string_axioms;
    proof { reveal_strlit("created"); reveal_strlit("reconstructed"); reveal_strlit("recovered"); reveal_strlit("cleaned"); reveal_strlit("observed"); reveal_strlit("rebound"); reveal_strlit("finished"); reveal_strlit("maintained"); reveal_strlit("maintenance-requested"); reveal_strlit("maintenance-idle-exit"); reveal_strlit("imported"); reveal_strlit("migrated"); reveal_strlit("owner-deployed"); reveal_strlit("used"); reveal_strlit("failed"); reveal_strlit(""); reveal_strlit("Created"); reveal_strlit("created\0"); }
    expect(policy::successful_report_status(Some("created")));
    expect(policy::successful_report_status(Some("reconstructed")));
    expect(policy::successful_report_status(Some("recovered")));
    expect(policy::successful_report_status(Some("cleaned")));
    expect(policy::successful_report_status(Some("observed")));
    expect(policy::successful_report_status(Some("rebound")));
    expect(policy::successful_report_status(Some("finished")));
    expect(policy::successful_report_status(Some("maintained")));
    expect(policy::successful_report_status(Some("maintenance-requested")));
    expect(policy::successful_report_status(Some("maintenance-idle-exit")));
    expect(policy::successful_report_status(Some("imported")));
    expect(policy::successful_report_status(Some("migrated")));
    expect(policy::successful_report_status(Some("owner-deployed")));
    expect(policy::successful_report_status(Some("used")));
    expect(!policy::successful_report_status(None));
    proof {
        assert("failed"@.len() == 6);
        assert(""@.len() == 0);
    }
    expect(!policy::successful_report_status(Some("failed")));
    expect(!policy::successful_report_status(Some("")));
    proof {
        assert("Created"@.len() == 7);
        assert("Created"@[0] != "created"@[0]);
        assert("Created"@[0] != "cleaned"@[0]);
        assert("Created"@[0] != "rebound"@[0]);
        assert("created\0"@.len() == 8);
        assert("created\0"@[0] != "observed"@[0]);
        assert("created\0"@[0] != "finished"@[0]);
        assert("created\0"@[0] != "imported"@[0]);
        assert("created\0"@[0] != "migrated"@[0]);
    }
    expect(!policy::successful_report_status(Some("Created")));
    expect(!policy::successful_report_status(Some("created\0")));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn empty_admission_has_no_policy_consumer_and_nonempty_enrollments_keep_the_check() {
    expect(!policy::enrolled_policy_required(0));
    expect(policy::enrolled_policy_required(1));
    expect(policy::enrolled_policy_required(usize::MAX));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn enclosing_fixture_requires_retired_objects_metadata_and_known_members() {
    expect(policy::producer_namespace_retired(true, true, true));
    expect(!policy::producer_namespace_retired(false, true, true));
    expect(!policy::producer_namespace_retired(true, false, true));
    expect(!policy::producer_namespace_retired(true, true, false));
    expect(!policy::producer_namespace_retired(false, false, false));
}
}
