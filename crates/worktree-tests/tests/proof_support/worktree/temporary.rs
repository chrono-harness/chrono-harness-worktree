#[cfg(verus_keep_ghost)]
use super::{expect, policy};

#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn rolling_windows_distinguish_failure_diagnostics_and_unknown_outcomes() {
    expect(policy::rolling_keep(true, false, false, 0, 86400, 3600) == 0);
    expect(policy::rolling_keep(true, true, false, 0, 86400, 3600) == 86400);
    expect(policy::rolling_keep(true, false, true, 0, 86400, 3600) == 86400);
    expect(policy::rolling_keep(false, false, false, 0, 86400, 3600) == 3600);
    expect(policy::rolling_keep(false, true, true, 0, 86400, 3600) == 3600);
    expect(policy::rolling_keep(true, true, true, u64::MAX, 0, u64::MAX) == 0);
}
}
