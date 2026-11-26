#[cfg(verus_keep_ghost)]
use vstd::contrib::exec_spec::ExecSpecEq;
#[cfg(verus_keep_ghost)]
use super::policy;

#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {
fn expect(actual: bool) requires actual { if !actual { panic!(); } }


#[cfg_attr(not(verus_keep_ghost), test)]
fn maintenance_keeps_coordinator_and_invoking_roots_in_order() {
    let roots = policy::maintenance_exclusions(17u64, u64::MAX);
    expect(roots.len() == 2);
    expect(roots[0] == 17);
    expect(roots[1] == u64::MAX);
    let same = policy::maintenance_exclusions(0u64, 0u64);
    expect(same.len() == 2);
    expect(same[0] == 0 && same[1] == 0);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn maintenance_keeps_complete_root_text_and_repeated_values() {
    let roots = policy::maintenance_exclusions(
        "/coordinator λ/source\n".to_string(), "/caller λ/source\0\n".to_string());
    expect(roots.len() == 2);
    expect(<&str>::exec_eq(roots[0].as_str(), "/coordinator λ/source\n"));
    expect(<&str>::exec_eq(roots[1].as_str(), "/caller λ/source\0\n"));
    let same = policy::maintenance_exclusions("".to_string(), "".to_string());
    expect(same.len() == 2);
    expect(<&str>::exec_eq(same[0].as_str(), ""));
    expect(<&str>::exec_eq(same[1].as_str(), ""));
}

fn retention_phase_case(complete: bool, unprotected: usize, succeeds: bool,
    payload: Option<u64>, message: &str, expected_stage: u8, expected_code: u64)
    requires if complete && unprotected == 0 {
        if succeeds { expected_stage == 2 && expected_code == 22 }
        else { expected_stage == 3 && expected_code == 23 }
    } else { expected_stage == 4 && expected_code == 24 },
{
    let attempt = |state: (u8, bool)| -> (out: ((u8, bool), Result<Option<u64>, String>))
        ensures out.0 == (1u8, state.1 && state.0 == 0u8),
            match out.1 {
                Ok(value) => succeeds && value == payload,
                Err(error) => !succeeds && error@ == message@,
            },
    {
        ((1, state.1 && state.0 == 0),
            if succeeds { Ok(payload) } else { Err(message.to_string()) })
    };
    let completed = |state: (u8, bool), value: Option<u64>| -> (out: ((u8, bool), u64))
        ensures out == ((2u8, state.1 && state.0 == 1u8 && value == payload), 22u64),
    { ((2, state.1 && state.0 == 1 && value == payload), 22) };
    let failed = |state: (u8, bool), error: String| -> (out: ((u8, bool), u64))
        ensures out == ((3u8, state.1 && state.0 == 1u8 && error@ == message@), 23u64),
    { ((3, state.1 && state.0 == 1 && <&str>::exec_eq(error.as_str(), message)), 23) };
    let deferred = |state: (u8, bool)| -> (out: ((u8, bool), u64))
        ensures out == ((4u8, state.1 && state.0 == 0u8), 24u64),
    { ((4, state.1 && state.0 == 0), 24) };
    let (state, code) = policy::maintenance_retention_phase(
        (0u8, true), complete, unprotected, attempt, completed, failed, deferred);
    expect(state.1);
    expect(state.0 == expected_stage);
    expect(code == expected_code);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn maintenance_retention_defers_before_access_when_original_protection_is_unknown() {
    retention_phase_case(false, 0, true, None, "unused", 4, 24);
    retention_phase_case(false, 0, false, Some(0), "unused error", 4, 24);
    retention_phase_case(false, 1, true, Some(u64::MAX), "unused", 4, 24);
    retention_phase_case(false, usize::MAX, false, None, "unused error", 4, 24);
    retention_phase_case(true, 1, true, None, "unused", 4, 24);
    retention_phase_case(true, usize::MAX, false, Some(u64::MAX), "unused error", 4, 24);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn maintenance_retention_preserves_observed_values_and_complete_errors() {
    retention_phase_case(true, 0, true, None, "unused", 2, 22);
    retention_phase_case(true, 0, true, Some(0), "unused", 2, 22);
    retention_phase_case(true, 0, true, Some(u64::MAX), "unused", 2, 22);
    retention_phase_case(true, 0, false, None, "", 3, 23);
    retention_phase_case(true, 0, false, Some(0), "read λ failed\0\n", 3, 23);
    retention_phase_case(true, 0, false, Some(u64::MAX), "second error", 3, 23);
}

// Each row specifies its own final observation, independent of the production predicates.
// Stages also record that every observation received its exact predecessor and address.
fn activation_case(query_result: (i32, i32), path: (bool, bool, bool, u32, u32),
    binding_result: (bool, bool, u64), message: &str, expected: (u8, bool, &str))
    requires match expected.0 {
        1 => query_result.0 == 0 && query_result.1 != 2 && !expected.1
            && expected.2@ == "OS activation requires a Unix datagram socket"@,
        2 => query_result.0 != 0 && !expected.1 && expected.2@ == message@,
        3 => query_result == (0i32, 2i32) && !path.0 && !expected.1
            && expected.2@ == message@,
        4 => query_result == (0i32, 2i32) && path.0 && !expected.1
            && ((!path.1 && expected.2@ == message@)
                || (path.1 && !path.2
                    && expected.2@ == "adopted activation path is not an owned socket"@)),
        5 => query_result == (0i32, 2i32) && path.0 && path.1 && path.2
            && ((path.3 != path.4 && !expected.1
                    && expected.2@ == "adopted activation path is not an owned socket"@)
                || (path.3 == path.4 && binding_result.0 && expected.1)),
        6 => query_result == (0i32, 2i32) && path.0 && path.1 && path.2 && path.3 == path.4
            && !binding_result.0 && binding_result.1 && expected.1,
        7 => query_result == (0i32, 2i32) && path.0 && path.1 && path.2 && path.3 == path.4
            && !binding_result.0 && !binding_result.1 && !expected.1
            && expected.2@ == message@,
        _ => false,
    },
{
    let query = |state: (u8, bool)| -> (out: ((u8, bool), (i32, i32)))
        ensures out == ((1u8, state.1 && state.0 == 0u8), query_result),
    { ((1, state.1 && state.0 == 0), query_result) };
    let last_error = |state: (u8, bool)| -> (out: ((u8, bool), String))
        ensures out.0 == (2u8, state.1 && state.0 == 1u8), out.1@ == message@,
    { ((2, state.1 && state.0 == 1), message.to_string()) };
    let address = |state: (u8, bool)| -> (out: ((u8, bool), Result<u64, String>))
        ensures out.0 == (3u8, state.1 && state.0 == 1u8),
            match out.1 {
                Ok(value) => path.0 && value == binding_result.2,
                Err(error) => !path.0 && error@ == message@,
            },
    {
        ((3, state.1 && state.0 == 1),
            if path.0 { Ok(binding_result.2) } else { Err(message.to_string()) })
    };
    let metadata = |state: (u8, bool)| -> (out: ((u8, bool), Result<(bool, u32), String>))
        ensures out.0 == (4u8, state.1 && state.0 == 3u8),
            match out.1 {
                Ok(fields) => path.1 && fields == (path.2, path.3),
                Err(error) => !path.1 && error@ == message@,
            },
    {
        ((4, state.1 && state.0 == 3),
            if path.1 { Ok((path.2, path.3)) } else { Err(message.to_string()) })
    };
    let user = |state: (u8, bool)| -> (out: ((u8, bool), u32))
        ensures out == ((5u8, state.1 && state.0 == 4u8), path.4),
    { ((5, state.1 && state.0 == 4), path.4) };
    let binding = |state: (u8, bool), address: &u64| -> (out: ((u8, bool), bool))
        ensures out == ((6u8, state.1 && state.0 == 5u8 && *address == binding_result.2),
            binding_result.1),
    { ((6, state.1 && state.0 == 5 && *address == binding_result.2), binding_result.1) };
    let describe = |state: (u8, bool), address: u64| -> (out: ((u8, bool), String))
        ensures out.0 == (7u8, state.1 && state.0 == 6u8 && address == binding_result.2),
            out.1@ == message@,
    { ((7, state.1 && state.0 == 6 && address == binding_result.2), message.to_string()) };
    let (state, result) = policy::validate_activation_sequence(
        (0u8, true), binding_result.0, 2i32, query, last_error,
        address, metadata, user, binding, describe);
    expect(state.1);
    expect(state.0 == expected.0);
    match result {
        Ok(()) => expect(expected.1),
        Err(error) => {
            expect(!expected.1);
            expect(<&str>::exec_eq(error.as_str(), expected.2));
        },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn activation_query_errors_keep_errno_text_and_stop() {
    activation_case((-1, 2), (false, false, false, 0, 1), (true, false, 0),
        "errno", (2, false, "errno"));
    activation_case((1, 1), (true, true, true, 0, 0), (false, true, u64::MAX),
        "", (2, false, ""));
    activation_case((i32::MIN, 2), (true, true, true, 0, 0), (true, true, 0),
        "原始\0错误\n", (2, false, "原始\0错误\n"));
    activation_case((i32::MAX, i32::MIN), (true, true, true, 0, 0), (false, true, 0),
        "query\n", (2, false, "query\n"));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn activation_requires_the_exact_datagram_kind_before_paths() {
    activation_case((0, 1), (false, false, false, 0, 1), (true, false, 0),
        "later", (1, false, "OS activation requires a Unix datagram socket"));
    activation_case((0, 0), (true, true, true, 0, 0), (false, true, 0),
        "later", (1, false, "OS activation requires a Unix datagram socket"));
    activation_case((0, -1), (true, true, true, 0, 0), (true, true, 0),
        "later", (1, false, "OS activation requires a Unix datagram socket"));
    activation_case((0, i32::MIN), (true, true, true, 0, 0), (false, true, 0),
        "later", (1, false, "OS activation requires a Unix datagram socket"));
    activation_case((0, i32::MAX), (true, true, true, 0, 0), (true, true, 0),
        "later", (1, false, "OS activation requires a Unix datagram socket"));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn activation_path_failures_preserve_order_and_complete_errors() {
    activation_case((0, 2), (false, false, false, 0, 1), (true, false, 0),
        "地址\0错误\n", (3, false, "地址\0错误\n"));
    activation_case((0, 2), (false, true, true, 0, 0), (false, true, u64::MAX),
        "", (3, false, ""));
    activation_case((0, 2), (true, false, false, 0, 1), (true, false, 0),
        "元数据\0错误\n", (4, false, "元数据\0错误\n"));
    activation_case((0, 2), (true, false, true, 0, 0), (false, true, u64::MAX),
        "", (4, false, ""));
    activation_case((0, 2), (true, true, false, 0, 0), (true, true, 0),
        "later", (4, false, "adopted activation path is not an owned socket"));
    activation_case((0, 2), (true, true, false, u32::MAX, 0), (false, true, 0),
        "later", (4, false, "adopted activation path is not an owned socket"));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn activation_named_sockets_still_require_exact_ownership() {
    activation_case((0, 2), (true, true, true, 0, u32::MAX), (true, true, 0),
        "later", (5, false, "adopted activation path is not an owned socket"));
    activation_case((0, 2), (true, true, true, u32::MAX, 0), (false, true, u64::MAX),
        "later", (5, false, "adopted activation path is not an owned socket"));
    activation_case((0, 2), (true, true, true, 0, 0), (true, false, 0),
        "must not describe", (5, true, ""));
    activation_case((0, 2), (true, true, true, u32::MAX, u32::MAX), (true, false, u64::MAX),
        "must not describe", (5, true, ""));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn activation_direct_binding_keeps_the_original_address_and_error() {
    activation_case((0, 2), (true, true, true, 0, 0), (false, true, 0),
        "must not describe", (6, true, ""));
    activation_case((0, 2), (true, true, true, u32::MAX, u32::MAX), (false, true, u64::MAX),
        "must not describe", (6, true, ""));
    activation_case((0, 2), (true, true, true, 7, 7), (false, false, u64::MAX),
        "原地址\0不匹配\n", (7, false, "原地址\0不匹配\n"));
    activation_case((0, 2), (true, true, true, 0, 0), (false, false, 0),
        "", (7, false, ""));
}

}
