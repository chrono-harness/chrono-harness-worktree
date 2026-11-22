#[cfg(verus_keep_ghost)]
use super::{expect, policy};

#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
#[cfg_attr(not(verus_keep_ghost), test)]
fn whole_directory_requires_owned_lifetime_and_consumer_exclusion() {
    expect(policy::rolling_disposal(true, true, true, true));
    expect(!policy::rolling_disposal(false, true, true, true));
    expect(!policy::rolling_disposal(true, false, true, true));
    expect(!policy::rolling_disposal(true, true, false, true));
    expect(!policy::rolling_disposal(true, true, true, false));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn rolling_deadline_caps_completion_and_metadata_windows() {
    expect(policy::clamp_rolling_deadline(1, 2, 4) == 3);
    expect(policy::clamp_rolling_deadline(1, 2, 3) == 3);
    expect(policy::clamp_rolling_deadline(1, 3, 3) == 3);
    expect(policy::clamp_rolling_deadline(4, 0, 3) == 3);
    expect(policy::clamp_rolling_deadline(u64::MAX - 1, 2, u64::MAX) == u64::MAX);
}
}
