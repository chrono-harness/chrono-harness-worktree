#[cfg(verus_keep_ghost)]
use super::{expect as expect_producer, policy as producer_policy};

#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
#[cfg(not(verus_keep_ghost))]
fn expect_producer(actual: bool) requires actual { if !actual { panic!(); } }
#[cfg_attr(not(verus_keep_ghost), test)]
fn producer_namespace_members_require_complete_literal_identity() {
    proof {
        reveal_strlit("objects.json"); reveal_strlit("producer-records");
        reveal_strlit("rolling-metadata"); reveal_strlit("rolling-disposals");
        reveal_strlit(""); reveal_strlit("unfinished-birth"); reveal_strlit("objects.json\0");
        assert("unfinished-birth"@[0] != "producer-records"@[0]);
        assert("unfinished-birth"@[0] != "rolling-metadata"@[0]);
        assert(""@.len() != "objects.json"@.len());
        assert(""@.len() != "producer-records"@.len());
        assert(""@.len() != "rolling-metadata"@.len());
        assert(""@.len() != "rolling-disposals"@.len());
        assert("objects.json\0"@.len() != "objects.json"@.len());
        assert("objects.json\0"@.len() != "producer-records"@.len());
        assert("objects.json\0"@.len() != "rolling-metadata"@.len());
        assert("objects.json\0"@.len() != "rolling-disposals"@.len());
    }
    use producer_policy::ProducerNamespaceMember::*;
    expect_producer(match producer_policy::producer_namespace_member("objects.json") { Owner => true, _ => false });
    expect_producer(match producer_policy::producer_namespace_member("producer-records") { Records => true, _ => false });
    expect_producer(match producer_policy::producer_namespace_member("rolling-metadata") { Metadata => true, _ => false });
    expect_producer(match producer_policy::producer_namespace_member("rolling-disposals") { Disposals => true, _ => false });
    expect_producer(match producer_policy::producer_namespace_member("") { Unknown => true, _ => false });
    expect_producer(match producer_policy::producer_namespace_member("unfinished-birth") { Unknown => true, _ => false });
    expect_producer(match producer_policy::producer_namespace_member("objects.json\0") { Unknown => true, _ => false });
}
}
