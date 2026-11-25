#[cfg(verus_keep_ghost)]
use vstd::contrib::exec_spec::ExecSpecEq;
#[cfg(verus_keep_ghost)]
use super::policy as absence_policy;


#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn imported_policy_source_preserves_explicit_field_and_only_uses_absent_field_source() {
    let policy = "policy";
    let source = "source";
    require_selected_source(absence_policy::imported_policy_source_commit(true, Some(policy), Some(source)), policy);
    require_selected_source(absence_policy::imported_policy_source_commit(true, Some(policy), None), policy);
    require_absence_expectation(absence_policy::imported_policy_source_commit(true, None, Some(source)).is_none());
    require_selected_source(absence_policy::imported_policy_source_commit(false, None, Some(source)), source);
    require_absence_expectation(absence_policy::imported_policy_source_commit(false, None, None).is_none());
}

fn require_selected_source(actual: Option<&str>, expected: &str)
    requires match actual { Some(actual) => actual@ == expected@, None => false },
{
    match actual {
        Some(actual) => require_absence_expectation(<&str>::exec_eq(actual, expected)),
        None => require_absence_expectation(false),
    }
}

fn require_capacity_fixture_exit(code: i32)
    requires code == 0,
    ensures code == 0,
{
    if code != 0 { panic!(); }
}

fn require_absence_expectation(actual: bool)
    requires actual,
{
    if !actual { panic!(); }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn interrupted_cleanup_custody_never_claims_a_producer_return() {
    require_absence_expectation(absence_policy::unfinished_cleanup_publication(false, false, false, false));
    require_absence_expectation(absence_policy::unfinished_cleanup_publication(true, true, true, false));
    require_absence_expectation(!absence_policy::unfinished_cleanup_publication(true, true, true, true));
    require_absence_expectation(!absence_policy::unfinished_cleanup_publication(true, true, false, false));
    require_absence_expectation(!absence_policy::unfinished_cleanup_publication(true, false, true, false));
    require_absence_expectation(!absence_policy::unfinished_cleanup_publication(true, false, false, true));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn cleanup_attempt_selection_keeps_unknown_age_and_current_responsibilities() {
    require_absence_expectation(!absence_policy::retain_cleanup_attempt(false, false, false));
    require_absence_expectation(absence_policy::retain_cleanup_attempt(false, false, true));
    require_absence_expectation(absence_policy::retain_cleanup_attempt(false, true, false));
    require_absence_expectation(absence_policy::retain_cleanup_attempt(false, true, true));
    require_absence_expectation(absence_policy::retain_cleanup_attempt(true, false, false));
    require_absence_expectation(absence_policy::retain_cleanup_attempt(true, false, true));
    require_absence_expectation(absence_policy::retain_cleanup_attempt(true, true, false));
    require_absence_expectation(absence_policy::retain_cleanup_attempt(true, true, true));
}

fn handoff_trace_step(trace: Vec<u8>, stage: u8, result: Result<(), String>)
    -> (out: (Vec<u8>, Result<(), String>))
    ensures out.0@ == trace@.push(stage), out.1 == result,
{
    let mut trace = trace;
    trace.push(stage);
    (trace, result)
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn ordinary_admission_selects_only_complete_registered_operation_names() {
    broadcast use vstd::string::group_string_axioms;
    proof {
        reveal_strlit("use"); reveal_strlit("check"); reveal_strlit("bootstrap");
        reveal_strlit("status"); reveal_strlit("maintain"); reveal_strlit("migrate");
        reveal_strlit("deploy-owner"); reveal_strlit("import");
        reveal_strlit(""); reveal_strlit("use\0");
        assert("use"@.len() == 3);
        assert("check"@.len() == 5);
        assert("bootstrap"@.len() == 9);
        assert("status"@.len() == 6);
        assert("maintain"@.len() == 8);
        assert("migrate"@.len() == 7);
        assert("deploy-owner"@.len() == 12);
        assert("import"@.len() == 6);
        assert(""@.len() == 0);
        assert("use\0"@.len() == 4);
    }
    require_absence_expectation(absence_policy::ordinary_admission("use"));
    require_absence_expectation(absence_policy::ordinary_admission("check"));
    require_absence_expectation(absence_policy::ordinary_admission("bootstrap"));
    require_absence_expectation(!absence_policy::ordinary_admission("status"));
    require_absence_expectation(!absence_policy::ordinary_admission("maintain"));
    require_absence_expectation(!absence_policy::ordinary_admission("migrate"));
    require_absence_expectation(!absence_policy::ordinary_admission("deploy-owner"));
    require_absence_expectation(!absence_policy::ordinary_admission("import"));
    require_absence_expectation(!absence_policy::ordinary_admission(""));
    require_absence_expectation(!absence_policy::ordinary_admission("use\0"));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn admission_handoff_orders_recovery_and_preserves_full_errors() {
    broadcast use vstd::array::group_array_axioms;
    let handoff_ok = |trace: Vec<u8>| -> (out: (Vec<u8>, Result<(), String>))
        ensures out.0@ == trace@.push(1), out.1 == Ok::<(), String>(()),
        { handoff_trace_step(trace, 1, Ok(())) };
    let handoff_error = |trace: Vec<u8>| -> (out: (Vec<u8>, Result<(), String>))
        ensures out.0@ == trace@.push(1),
            match out.1 { Err(error) => error@ == "原交接错误\0完整"@, Ok(()) => false },
        { handoff_trace_step(trace, 1, Err("原交接错误\0完整".to_string())) };
    let reload_ok = |trace: Vec<u8>, _error: String| -> (out: (Vec<u8>, Result<(), String>))
        ensures out.0@ == trace@.push(2), out.1 == Ok::<(), String>(()),
        { handoff_trace_step(trace, 2, Ok(())) };
    let reload_error = |trace: Vec<u8>, error: String| -> (out: (Vec<u8>, Result<(), String>))
        ensures out.0@ == trace@.push(2), out.1 == Err::<(), String>(error),
        { handoff_trace_step(trace, 2, Err(error)) };
    let protect_ok = |trace: Vec<u8>| -> (out: (Vec<u8>, Result<(), String>))
        ensures out.0@ == trace@.push(3), out.1 == Ok::<(), String>(()),
        { handoff_trace_step(trace, 3, Ok(())) };
    let protect_error = |trace: Vec<u8>| -> (out: (Vec<u8>, Result<(), String>))
        ensures out.0@ == trace@.push(3),
            match out.1 { Err(error) => error@ == "必要保护失败\0完整"@, Ok(()) => false },
        { handoff_trace_step(trace, 3, Err("必要保护失败\0完整".to_string())) };
    let (trace, result) = absence_policy::admission_handoff(Vec::<u8>::new(),
        handoff_ok, reload_error, protect_ok);
    assert(trace@ =~= seq![1u8, 3]);
    require_absence_expectation(<&[u8]>::exec_eq(&trace, &[1u8, 3]));
    require_absence_expectation(result.is_ok());

    let (trace, result) = absence_policy::admission_handoff(Vec::<u8>::new(),
        handoff_error, reload_ok, protect_ok);
    assert(trace@ =~= seq![1u8, 2, 3]);
    require_absence_expectation(<&[u8]>::exec_eq(&trace, &[1u8, 2, 3]));
    require_absence_expectation(result.is_ok());

    let (trace, result) = absence_policy::admission_handoff(Vec::<u8>::new(),
        handoff_error, reload_error, protect_ok);
    assert(trace@ =~= seq![1u8, 2]);
    require_absence_expectation(<&[u8]>::exec_eq(&trace, &[1u8, 2]));
    match result {
        Err(error) => require_absence_expectation(<&str>::exec_eq(&error, "原交接错误\0完整")),
        Ok(()) => require_absence_expectation(false),
    }

    let (trace, result) = absence_policy::admission_handoff(Vec::<u8>::new(),
        handoff_ok, reload_ok, protect_error);
    assert(trace@ =~= seq![1u8, 3]);
    require_absence_expectation(<&[u8]>::exec_eq(&trace, &[1u8, 3]));
    match result {
        Err(error) => require_absence_expectation(<&str>::exec_eq(&error, "必要保护失败\0完整")),
        Ok(()) => require_absence_expectation(false),
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn enrollment_publication_skips_only_the_shrinking_preparation_and_stops_on_first_error() {
    broadcast use vstd::array::group_array_axioms;
    let before_ok = |trace: Vec<u8>| -> (out: (Vec<u8>, Result<(), String>))
        ensures out.0@ == trace@.push(1), out.1 == Ok::<(), String>(()),
        { handoff_trace_step(trace, 1, Ok(())) };
    let before_error = |trace: Vec<u8>| -> (out: (Vec<u8>, Result<(), String>))
        ensures out.0@ == trace@.push(1),
            match out.1 { Err(error) => error@ == "新增引用保护失败\0原件"@, Ok(()) => false },
        { handoff_trace_step(trace, 1, Err("新增引用保护失败\0原件".to_string())) };
    let forbidden = |trace: Vec<u8>| -> (out: (Vec<u8>, Result<(), String>))
        ensures out.0@ == trace@.push(1),
            match out.1 { Err(error) => error@ == "不得执行"@, Ok(()) => false },
        { handoff_trace_step(trace, 1, Err("不得执行".to_string())) };
    let write_ok = |trace: Vec<u8>| -> (out: (Vec<u8>, Result<(), String>))
        ensures out.0@ == trace@.push(2), out.1 == Ok::<(), String>(()),
        { handoff_trace_step(trace, 2, Ok(())) };
    let write_error = |trace: Vec<u8>| -> (out: (Vec<u8>, Result<(), String>))
        ensures out.0@ == trace@.push(2),
            match out.1 { Err(error) => error@ == "账本写入失败\0原件"@, Ok(()) => false },
        { handoff_trace_step(trace, 2, Err("账本写入失败\0原件".to_string())) };
    let after_ok = |trace: Vec<u8>| -> (out: (Vec<u8>, Result<(), String>))
        ensures out.0@ == trace@.push(3), out.1 == Ok::<(), String>(()),
        { handoff_trace_step(trace, 3, Ok(())) };
    let after_error = |trace: Vec<u8>| -> (out: (Vec<u8>, Result<(), String>))
        ensures out.0@ == trace@.push(3),
            match out.1 { Err(error) => error@ == "引用收窄失败\0原件"@, Ok(()) => false },
        { handoff_trace_step(trace, 3, Err("引用收窄失败\0原件".to_string())) };
    let (trace, result) = absence_policy::publish_enrollment_record(Vec::<u8>::new(), true,
        before_ok, write_ok, after_ok);
    assert(trace@ =~= seq![1u8, 2, 3]);
    require_absence_expectation(<&[u8]>::exec_eq(&trace, &[1u8, 2, 3]));
    require_absence_expectation(result.is_ok());
    let (trace, result) = absence_policy::publish_enrollment_record(Vec::<u8>::new(), false,
        forbidden, write_ok, after_ok);
    assert(trace@ =~= seq![2u8, 3]);
    require_absence_expectation(<&[u8]>::exec_eq(&trace, &[2u8, 3]));
    require_absence_expectation(result.is_ok());
    let (trace, result) = absence_policy::publish_enrollment_record(Vec::<u8>::new(), true,
        before_error, write_ok, after_ok);
    assert(trace@ =~= seq![1u8]);
    let expected = [1u8];
    assert(expected@ =~= seq![1u8]);
    require_absence_expectation(<&[u8]>::exec_eq(&trace, &expected));
    match result {
        Err(error) => require_absence_expectation(<&str>::exec_eq(&error, "新增引用保护失败\0原件")),
        Ok(()) => require_absence_expectation(false),
    }
    let (trace, result) = absence_policy::publish_enrollment_record(Vec::<u8>::new(), false,
        before_ok, write_error, after_ok);
    assert(trace@ =~= seq![2u8]);
    let expected = [2u8];
    assert(expected@ =~= seq![2u8]);
    require_absence_expectation(<&[u8]>::exec_eq(&trace, &expected));
    match result {
        Err(error) => require_absence_expectation(<&str>::exec_eq(&error, "账本写入失败\0原件")),
        Ok(()) => require_absence_expectation(false),
    }
    let (trace, result) = absence_policy::publish_enrollment_record(Vec::<u8>::new(), false,
        before_ok, write_ok, after_error);
    assert(trace@ =~= seq![2u8, 3]);
    require_absence_expectation(<&[u8]>::exec_eq(&trace, &[2u8, 3]));
    match result {
        Err(error) => require_absence_expectation(<&str>::exec_eq(&error, "引用收窄失败\0原件")),
        Ok(()) => require_absence_expectation(false),
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn absent_cache_probe_requires_maintenance_pending_cache_and_the_original_exclusion() {
    require_absence_expectation(absence_policy::absence_probe_required(true, true, true));
    require_absence_expectation(!absence_policy::absence_probe_required(false, true, true));
    require_absence_expectation(!absence_policy::absence_probe_required(true, false, true));
    require_absence_expectation(!absence_policy::absence_probe_required(true, true, false));
    require_absence_expectation(!absence_policy::absence_probe_required(false, false, true));
    require_absence_expectation(!absence_policy::absence_probe_required(false, true, false));
    require_absence_expectation(!absence_policy::absence_probe_required(true, false, false));
    require_absence_expectation(!absence_policy::absence_probe_required(false, false, false));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn absent_cache_observation_preserves_unknown_task_and_unclaimed_removal() {
    let observation = absence_policy::missing_cache_observation();
    require_absence_expectation(<&str>::exec_eq(observation.status, "preserved"));
    require_absence_expectation(<&str>::exec_eq(observation.checkout_presence, "absent"));
    require_absence_expectation(<&str>::exec_eq(observation.metadata_presence, "absent"));
    require_absence_expectation(<&str>::exec_eq(observation.task_completion, "unknown"));
    require_absence_expectation(<&str>::exec_eq(observation.worktree_removal, "not-claimed"));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn absent_enrollment_requires_exact_registered_kind_and_no_other_attachment() {
    broadcast use vstd::string::group_string_axioms;
    proof {
        reveal_strlit("birth"); reveal_strlit("import"); reveal_strlit("");
        reveal_strlit("birth\0"); reveal_strlit("HEAD"); reveal_strlit("HEAD\0");
        reveal_strlit("refs/heads/task");
        assert(""@.len() != "birth"@.len());
        assert(""@.len() != "import"@.len());
        assert("birth\0"@.len() != "birth"@.len());
        assert("birth\0"@[0] != "import"@[0]);
        assert("refs/heads/task"@.len() != "HEAD"@.len());
        assert("HEAD\0"@.len() != "HEAD"@.len());
    }
    require_absence_expectation(absence_policy::known_absent_enrollment(false, "birth"));
    require_absence_expectation(absence_policy::known_absent_enrollment(false, "import"));
    require_absence_expectation(!absence_policy::known_absent_enrollment(true, "birth"));
    require_absence_expectation(!absence_policy::known_absent_enrollment(true, "import"));
    require_absence_expectation(!absence_policy::known_absent_enrollment(false, ""));
    require_absence_expectation(!absence_policy::known_absent_enrollment(false, "birth\0"));
    require_absence_expectation(absence_policy::absent_attachment_conflict(true, "HEAD", false));
    require_absence_expectation(absence_policy::absent_attachment_conflict(true, "HEAD", true));
    require_absence_expectation(!absence_policy::absent_attachment_conflict(false, "HEAD", true));
    require_absence_expectation(!absence_policy::absent_attachment_conflict(false, "HEAD", false));
    require_absence_expectation(absence_policy::absent_attachment_conflict(false, "refs/heads/task", true));
    require_absence_expectation(!absence_policy::absent_attachment_conflict(false, "refs/heads/task", false));
    require_absence_expectation(absence_policy::absent_attachment_conflict(false, "HEAD\0", true));
}

}
