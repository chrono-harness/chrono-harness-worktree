#[cfg(verus_keep_ghost)]
use super::policy;

#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {
#[cfg_attr(not(verus_keep_ghost), test)]
fn repeated_actions_keep_every_original_index_in_order() {
    let indices = policy::append_action_index(Vec::new(), true, 0);
    let indices = policy::append_action_index(indices, false, 1);
    let indices = policy::append_action_index(indices, true, 2);
    if indices.len() != 2 || indices[0] != 0 || indices[1] != 2 { panic!(); }
}
}
