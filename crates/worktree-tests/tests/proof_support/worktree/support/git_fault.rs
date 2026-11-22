#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {
pub fn inventory_failure<T>(error: Option<T>) -> (out: Result<(), T>)
    ensures match error {
        None => out == Ok::<(), T>(()),
        Some(original) => out == Err::<(), T>(original),
    },
{
    match error { None => Ok(()), Some(original) => Err(original) }
}

// The Rust command-boundary cases already check the original stderr, exit and unknown inventory.
#[cfg(verus_keep_ghost)]
fn configured_inventory_failure_preserves_the_original_and_absence_can_continue() {
    match inventory_failure::<u8>(None) { Ok(()) => {}, Err(_) => panic!() }
    match inventory_failure(Some(74u8)) { Err(original) => { if original != 74 { panic!(); } }, Ok(()) => panic!() }
}
}
