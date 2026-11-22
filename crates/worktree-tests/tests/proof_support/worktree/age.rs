#[cfg(verus_keep_ghost)]
use super::{expect, policy};

#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {
#[cfg_attr(not(verus_keep_ghost), test)]
fn age_preservation_is_skipped_only_without_disposal_and_with_retained_file() {
    expect(!policy::age_preservation_required(false, true));
    expect(policy::age_preservation_required(false, false));
    expect(policy::age_preservation_required(true, false));
    expect(policy::age_preservation_required(true, true));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn only_actual_unreleased_evidence_blocks_checkout_removal() {
    let needed = policy::retain_evidence(false, true);
    let absent = policy::retain_evidence(false, false);
    let released = policy::retain_evidence(true, true);
    let released_absent = policy::retain_evidence(true, false);
    expect(needed && !absent && !released && !released_absent);
}

pub open spec fn caller_observation_matches(expected: (String, String),
    result: (i32, String), observed: Result<(String, String, bool), String>,
    out: Result<(), String>) -> bool
{
    match observed {
        Err(error) => out == Err::<(), String>(error),
        Ok(fields) => out.is_ok() == (result.0 == 0 && result.1@ == expected.0@
            && fields.0@ == "unique caller source λ\n"@
            && fields.1@ == expected.1@ && !fields.2),
    }
}

// The same driver sequences the real fixture callbacks and the independent cases.
// Callback failures remain failures; only the observed values determine acceptance.
fn linked_caller_regression<C, G, B, R, O, P>(gate: G, build: B, run: R, observe: O, preserve: P)
    -> (out: Result<(), String>)
    where G: FnOnce() -> bool, B: FnOnce() -> (C, (String, String)),
        R: FnOnce(C) -> (C, (i32, String)),
        O: FnOnce(&C) -> Result<(String, String, bool), String>, P: FnOnce(C),
    requires gate.requires(()), build.requires(()),
        forall|state: C| run.requires((state,)),
        forall|state: &C| observe.requires((state,)),
        forall|state: C| preserve.requires((state,)),
    ensures (gate.ensures((), false) && out == Ok::<(), String>(()))
        || (gate.ensures((), true) && exists|prepared: C, expected: (String, String)|
            #[trigger] build.ensures((), (prepared, expected))
            && exists|executed: C, result: (i32, String)|
                #[trigger] run.ensures((prepared,), (executed, result))
                && exists|observed: Result<(String, String, bool), String>|
                    #[trigger] observe.ensures((&executed,), observed)
                    && caller_observation_matches(expected, result, observed, out)
                    && (out.is_err() ==> preserve.ensures((executed,), ()))),
{
    let active = gate();
    assert(gate.ensures((), active));
    if !active { return Ok(()); }
    let (state, expected) = build();
    let ghost prepared = state;
    let ghost original = expected;
    assert(build.ensures((), (prepared, original)));
    let (state, result) = run(state);
    let ghost executed = state;
    let ghost returned = result;
    assert(run.ensures((prepared,), (executed, returned)));
    let observed = observe(&state);
    let ghost observation = observed;
    assert(observe.ensures((&executed,), observation));
    let out = match observed {
        Err(error) => Err(error),
        Ok(fields) => {
            if result.0 == 0 && policy::equal_text(result.1.as_str(), expected.0.as_str())
                && policy::equal_text(fields.0.as_str(), "unique caller source λ\n")
                && policy::equal_text(fields.1.as_str(), expected.1.as_str()) && !fields.2 {
                Ok(())
            } else {
                Err("linked maintenance changed its caller or failed to retire its sibling".to_string())
            }
        },
    };
    assert(caller_observation_matches(original, returned, observation, out));
    if out.is_err() {
        preserve(state);
        assert(preserve.ensures((executed,), ()));
    }
    out
}

fn caller_case(code: i32, reported: &str, source: &str, head: &str,
    sibling: bool, failure: bool, expected: bool)
    requires expected == (!failure && code == 0 && reported@ == "/caller λ"@
        && source@ == "unique caller source λ\n"@ && head@ == "original head"@ && !sibling),
{
    let build = || -> (out: (u8, (String, String)))
        ensures out.0 == 1, out.1.0@ == "/caller λ"@, out.1.1@ == "original head"@,
    { (1, ("/caller λ".to_string(), "original head".to_string())) };
    let run = |stage: u8| -> (out: (u8, (i32, String)))
        ensures out.0 == (if stage == 1 { 2u8 } else { 0u8 }),
            out.1.0 == code, out.1.1@ == reported@,
    { (if stage == 1 { 2 } else { 0 }, (code, reported.to_string())) };
    let observe = |stage: &u8| -> (out: Result<(String, String, bool), String>)
        ensures match out {
            Err(error) => (failure || *stage != 2) && error@ == "observation λ\0\n"@,
            Ok(fields) => !failure && *stage == 2 && fields.0@ == source@
                && fields.1@ == head@ && fields.2 == sibling,
        },
    {
        if failure || *stage != 2 { Err("observation λ\0\n".to_string()) }
        else { Ok((source.to_string(), head.to_string(), sibling)) }
    };
    let gate = || -> (active: bool) ensures active { true };
    let preserve = |stage: u8| { let _ = stage; };
    let result = linked_caller_regression(gate, build, run, observe, preserve);
    expect(result.is_ok() == expected);
    if failure {
        match result {
            Err(error) => expect(policy::equal_text(error.as_str(), "observation λ\0\n")),
            Ok(()) => expect(false),
        }
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn linked_caller_verdict_requires_each_original_observation() {
    proof {
        reveal_strlit("/caller λ"); reveal_strlit("/other");
        reveal_strlit("unique caller source λ\n"); reveal_strlit("changed");
        reveal_strlit("original head");
        assert("/caller λ"@.len() != "/other"@.len());
        assert("unique caller source λ\n"@.len() != "changed"@.len());
        assert("original head"@.len() != "changed"@.len());
    }
    caller_case(0, "/caller λ", "unique caller source λ\n", "original head", false, false, true);
    caller_case(1, "/caller λ", "unique caller source λ\n", "original head", false, false, false);
    caller_case(0, "/other", "unique caller source λ\n", "original head", false, false, false);
    caller_case(0, "/caller λ", "changed", "original head", false, false, false);
    caller_case(0, "/caller λ", "unique caller source λ\n", "changed", false, false, false);
    caller_case(0, "/caller λ", "unique caller source λ\n", "original head", true, false, false);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn linked_caller_verdict_preserves_observation_failures() {
    caller_case(0, "/caller λ", "unique caller source λ\n", "original head", false, true, false);
    caller_case(i32::MIN, "", "", "", true, true, false);
    caller_case(i32::MAX, "/caller λ\0", "unique caller source λ", "original head\n", true, true, false);
}
}
