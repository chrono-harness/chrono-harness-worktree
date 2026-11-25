#[cfg(verus_keep_ghost)]
use super::policy as input_policy;

#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {
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
