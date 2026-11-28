#[cfg(verus_keep_ghost)]
use super::policy as input_policy;
#[cfg(not(verus_keep_ghost))]
use chrono_worktree::verified_lifecycle as input_policy;
#[cfg(not(verus_keep_ghost))]
use vstd::prelude::verus;

#[cfg(verus_keep_ghost)]
use vstd::{contrib::exec_spec::ExecSpecEq, prelude::*};

#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {
#[cfg(verus_keep_ghost)]
fn the_profile_and_birth_must_authorize_full_context() {
    use super::expect;
    use input_policy::check_inputs::{context_originals, full_check_birth, full_check_origin, scoped_check_profile};
    broadcast use vstd::array::group_array_axioms;
    proof {
        reveal_strlit("full");
        reveal_strlit("chrono-ci-check/v1");
        reveal_strlit("chrono-ci-check/v2");
        reveal_strlit("chrono-ci-check/v3");
        reveal_strlit("feature");
        reveal_strlit("integration");
        reveal_strlit("delivery");
        assert("full"@.len() != "chrono-ci-check/v1"@.len()
            && "full"@.len() != "chrono-ci-check/v2"@.len()
            && "full"@.len() != "chrono-ci-check/v3"@.len());
        assert("feature"@.len() != "integration"@.len() && "feature"@.len() != "delivery"@.len());
    }
    expect(scoped_check_profile(Some("chrono-ci-check/v1")));
    expect(scoped_check_profile(Some("chrono-ci-check/v2")));
    expect(scoped_check_profile(Some("chrono-ci-check/v3")));
    expect(!scoped_check_profile(Some("full")));
    expect(!scoped_check_profile(None));
    let origin_schema = Some("chrono-worktree-origin/v1");
    expect(full_check_origin(origin_schema, true, Some("integration")));
    expect(full_check_origin(origin_schema, true, Some("delivery")));
    expect(!full_check_origin(origin_schema, true, Some("feature")));
    expect(!full_check_origin(origin_schema, false, Some("delivery")));
    expect(!full_check_origin(None, true, Some("delivery")));
    expect(full_check_birth(Some("created"), true, true, true));
    expect(full_check_birth(Some("reconstructed"), true, true, true));
    expect(!full_check_birth(None, true, true, true));
    expect(!full_check_birth(Some("created"), false, true, true));
    expect(!full_check_birth(Some("created"), true, false, true));
    expect(!full_check_birth(Some("created"), true, true, false));
    let paths = context_originals(Some("context"), None, Some("birth"));
    expect(matches!(paths[0], Some(path) if <&str>::exec_eq(path, "context")));
    expect(paths[1].is_none());
    expect(matches!(paths[2], Some(path) if <&str>::exec_eq(path, "birth")));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn presence_hash_and_length_must_stay_together() {
    proof {
        reveal_strlit("a"); reveal_strlit("bb");
        assert("a"@.len() != "bb"@.len());
    }
    let before = Some(("a", 7u64));
    if !input_policy::unchanged_input(None, None)
        || !input_policy::unchanged_input(before, Some(("a", 7)))
        || input_policy::unchanged_input(None, before)
        || input_policy::unchanged_input(before, None)
        || input_policy::unchanged_input(before, Some(("bb", 7)))
        || input_policy::unchanged_input(before, Some(("a", 8))) { panic!(); }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn policy_requires_a_dedicated_state_directory() {
    broadcast use vstd::array::group_array_axioms;
    proof {
        reveal_strlit("versions"); reveal_strlit("install"); reveal_strlit(".git");
        reveal_strlit(""); reveal_strlit("."); reveal_strlit("..");
        reveal_strlit(".chrono-harness"); reveal_strlit("state"); reveal_strlit("other");
        assert("state"@[0] != "other"@[0]);
        assert("install"@.len() == 7 && "versions"@.len() == 8);
        assert(""@.len() == 0 && "."@.len() == 1 && ".."@.len() == 2 && ".git"@.len() == 4);
        assert(input_policy::usable_input_component("install"));
        assert(input_policy::usable_input_component("versions"));
    }
    let valid = [".chrono-harness", "state", "install", "versions"];
    let root = [".chrono-harness", "state", "versions"];
    let reserved = [".chrono-harness", "state", ".git", "versions"];
    let empty = [".chrono-harness", "state", "", "versions"];
    let outside = [".chrono-harness", "other", "install", "versions"];
    let good = input_policy::dedicated_input_path(&valid);
    let at_root = input_policy::dedicated_input_path(&root);
    let git = input_policy::dedicated_input_path(&reserved);
    let blank = input_policy::dedicated_input_path(&empty);
    let elsewhere = input_policy::dedicated_input_path(&outside);
    proof {
        assert forall|i: int| 2 <= i < valid@.len() implies
            input_policy::usable_input_component(#[trigger] valid@[i]) by {
            if i == 2 { assert(valid@[i] == "install"); }
            else { assert(valid@[i] == "versions"); }
        }
        assert(reserved@[2] == ".git");
        assert(empty@[2] == "");
        assert(good);
        assert(!at_root);
        assert(!git);
        assert(!blank);
        assert(!elsewhere);
    }
    if !good || at_root || git || blank || elsewhere { panic!(); }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn only_the_named_regular_input_is_accepted() {
    proof {
        reveal_strlit("versions"); reveal_strlit("versions.copy");
        assert("versions"@.len() != "versions.copy"@.len());
    }
    if !input_policy::accepts_input_entry("versions", "versions", true)
        || input_policy::accepts_input_entry("versions", "versions", false)
        || input_policy::accepts_input_entry("versions", "versions.copy", true) { panic!(); }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn full_check_artifacts_have_separate_paths() {
    proof {
        reveal_strlit("a"); reveal_strlit("bb"); reveal_strlit("ccc");
        assert("a"@.len() != "bb"@.len());
        assert("a"@.len() != "ccc"@.len());
        assert("bb"@.len() != "ccc"@.len());
    }
    let ok = input_policy::full_paths_distinct("a", "bb", "ccc");
    let origin_context = input_policy::full_paths_distinct("a", "a", "ccc");
    let origin_manifest = input_policy::full_paths_distinct("a", "bb", "a");
    let context_manifest = input_policy::full_paths_distinct("a", "bb", "bb");
    if !ok || origin_context || origin_manifest || context_manifest { panic!(); }
}
}
