//! A pre-fetch identity does not assert that fetching completed or identify its result.
use vstd::contrib::exec_spec::ExecSpecEq;
use vstd::prelude::*;

verus! {
/// A present base must be a string naming this commit; a malformed field is a mismatch.
pub(crate) fn unresolved_fetch_report(
    reference: &str, expected: &str, removed: Option<bool>,
    base: Option<Option<&str>>, head: &str,
) -> (accepted: bool)
    ensures accepted == (reference@ == expected@ && removed == Some(false)
        && match base {
            None => true,
            Some(Some(value)) => value@ == head@,
            Some(None) => false,
        }),
{
    <&str>::exec_eq(reference, expected) && removed == Some(false) && match base {
        None => true,
        Some(Some(value)) => <&str>::exec_eq(value, head),
        Some(None) => false,
    }
}
}

#[cfg(not(verus_keep_ghost))]
use crate::document::decode;
#[cfg(not(verus_keep_ghost))]
use crate::facts;
#[cfg(not(verus_keep_ghost))]
use crate::recovery::{self, Evidence, ExpectedResult};
#[cfg(not(verus_keep_ghost))]
use serde::Deserialize;
#[cfg(not(verus_keep_ghost))]
use serde_json::Value;
#[cfg(not(verus_keep_ghost))]
use std::path::Path;

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Intent {
    schema: String,
    operation: String,
    source_root: String,
    source_commit: String,
    source_registry_digest: String,
    config_path: String,
    config_sha256: String,
    fetch_ref: String,
    remote: String,
    target_ref: String,
    lock_reason: String,
    report_path: String,
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn publish(root: &Path, report: &mut Value) -> Result<(), String> {
    recovery::publish_fields(
        root,
        report,
        "chrono-worktree-fetch-intent/v1",
        ".fetch-intent.json",
        "fetch_intent",
        &[
            "operation",
            "source_root",
            "source_commit",
            "source_registry_digest",
            "config_path",
            "config_sha256",
            "fetch_ref",
            "remote",
            "target_ref",
            "lock_reason",
            "report_path",
        ],
    )
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn inspect(
    root: &Path,
    config_path: &str,
    config_bytes: &[u8],
    path: &str,
    digest: &str,
    expected: &ExpectedResult,
    report: &mut Value,
) -> Result<Evidence, String> {
    let (bytes, descriptor) = recovery::read_intent(root, path, digest, report)?;
    let intent: Intent = decode(&bytes)?;
    let config: crate::Config = decode(config_bytes)?;
    if intent.schema != "chrono-worktree-fetch-intent/v1"
        || !matches!(intent.operation.as_str(), "start" | "reconstruct")
        || intent.source_root != root.to_str().ok_or("source root is not UTF-8")?
        || intent.config_path != config_path
        || intent.config_sha256 != crate::document::sha256(config_bytes)
        || intent.source_registry_digest.is_empty()
        || intent.remote != config.remote
        || !intent.target_ref.starts_with("refs/heads/")
        || !intent
            .lock_reason
            .starts_with(&format!("{}-", intent.operation))
        || intent.fetch_ref != format!("refs/chrono-harness/fetch/{}", intent.lock_reason)
        || intent.report_path != format!("{}{}.json", config.report_directory, intent.lock_reason)
        || path != format!("{}.fetch-intent.json", intent.report_path)
    {
        return Err("interrupted-fetch/configuration identity mismatch".into());
    }
    facts::full_oid(&intent.source_commit)?;
    recovery::observe_result(
        root,
        path,
        bytes,
        descriptor,
        &intent.report_path,
        expected,
        report,
    )
}
