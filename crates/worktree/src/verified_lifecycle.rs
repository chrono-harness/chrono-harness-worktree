//! 生命周期选择直接供清理所有者调用；文件存在性与租约由外部接口提供。
use vstd::contrib::exec_spec::ExecSpecEq;
#[cfg(not(verus_keep_ghost))]
use vstd::prelude::verus;
#[cfg(verus_keep_ghost)]
#[path = "child_io.rs"]
pub(crate) mod child_io;
#[cfg(verus_keep_ghost)]
#[path = "ownership.rs"]
pub(crate) mod ownership;
#[cfg(verus_keep_ghost)]
#[path = "process_fds.rs"]
pub(crate) mod process_fds;
#[cfg(verus_keep_ghost)]
#[path = "removal.rs"]
pub(crate) mod removal;
#[cfg(verus_keep_ghost)]
#[path = "retention_session.rs"]
pub(crate) mod retention_session;

#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
#[path = "automatic/producer/body.rs"]
pub mod copied_body;

#[cfg(verus_keep_ghost)]
#[path = "document.rs"]
mod document;
#[cfg(not(verus_keep_ghost))]
pub use crate::document::add_stream_bytes;
#[cfg(verus_keep_ghost)]
pub use document::add_stream_bytes;

#[cfg(verus_keep_ghost)]
#[path = "automatic/operation_scopes.rs"]
pub mod operation_scopes;

#[cfg(verus_keep_ghost)]
#[path = "automatic/standalone_expiry.rs"]
mod standalone_expiry;
#[cfg(verus_keep_ghost)]
pub use standalone_expiry::report_owner_matches;

#[cfg(verus_keep_ghost)]
#[path = "remote.rs"]
pub mod remote;
#[cfg(not(verus_keep_ghost))]
pub use crate::remote::{Advertised, already_absent, record_ref, retained_branch, single_endpoint};
#[cfg(all(not(verus_keep_ghost), feature = "remote-observation-test"))]
pub use crate::remote::{RemoteCheck, RemoteSelection};
#[cfg(verus_keep_ghost)]
pub use remote::{Advertised, already_absent, record_ref, retained_branch, single_endpoint};

#[cfg(verus_keep_ghost)]
#[path = "maintenance_signal.rs"]
pub(crate) mod maintenance_signal;

#[cfg(verus_keep_ghost)]
#[path = "maintenance.rs"]
mod maintenance_verification;
#[cfg(not(verus_keep_ghost))]
pub use crate::maintenance::{removed_checkout, retained_reference_ancestor_required};
#[cfg(verus_keep_ghost)]
pub use maintenance_verification::{removed_checkout, retained_reference_ancestor_required};

#[cfg(verus_keep_ghost)]
#[path = "check_inputs.rs"]
mod check_inputs;
#[cfg(not(verus_keep_ghost))]
pub use crate::check_inputs::{
    accepts_input_entry, dedicated_input_path, full_paths_distinct, unchanged_input,
};
#[cfg(verus_keep_ghost)]
pub use check_inputs::usable_input_component;
#[cfg(verus_keep_ghost)]
pub use check_inputs::{
    accepts_input_entry, dedicated_input_path, full_paths_distinct, unchanged_input,
};

#[cfg(verus_keep_ghost)]
#[path = "start.rs"]
mod start_verification;
#[cfg(not(verus_keep_ghost))]
pub use crate::start::select_report_input_scope;
#[cfg(verus_keep_ghost)]
pub use start_verification::select_report_input_scope;

#[cfg(verus_keep_ghost)]
#[path = "automatic.rs"]
// The ordinary automatic module owns formatting of its runtime child modules.
// This proof-only alias has a different module directory and must not traverse them.
#[rustfmt::skip]
mod automatic_verification;
#[cfg(not(verus_keep_ghost))]
pub use crate::automatic::{
    MissingCacheObservation, absence_probe_required, absent_attachment_conflict, admission_handoff,
    known_absent_enrollment, missing_cache_observation, publish_enrollment_record,
    retain_cleanup_attempt, unfinished_cleanup_publication,
};
#[cfg(verus_keep_ghost)]
pub use automatic_verification::{
    MissingCacheObservation, absence_probe_required, absent_attachment_conflict, admission_handoff,
    known_absent_enrollment, missing_cache_observation, publish_enrollment_record,
    retain_cleanup_attempt, unfinished_cleanup_publication,
};

#[cfg(not(verus_keep_ghost))]
pub use crate::automatic::age::{
    admits_age_policy, age_preservation_required, age_window_passed, reusable_release,
};
#[cfg(verus_keep_ghost)]
pub use automatic_verification::age::{
    admits_age_policy, age_preservation_required, age_window_passed, reusable_release,
};

#[cfg(not(verus_keep_ghost))]
pub use crate::automatic::cache_adoption::{
    current_main_custody, custody_declaration, custody_generation, custody_paths, main_cache_target,
};
#[cfg(verus_keep_ghost)]
pub use automatic_verification::cache_adoption::{
    current_main_custody, custody_declaration, custody_generation, custody_paths, main_cache_target,
};

#[cfg(not(verus_keep_ghost))]
pub use crate::automatic::inventory::append_action_index;
#[cfg(not(verus_keep_ghost))]
pub use crate::automatic::migration::{
    adopted_policy_binding_error, checkout_migration_ready, policy_migration_decision,
    policy_state_conflict, policy_state_present, store_current_policy,
};
#[cfg(not(verus_keep_ghost))]
pub use crate::automatic::service::{
    maintenance_exclusions, maintenance_has_events, maintenance_retention_phase, next_enrollment,
    validate_activation_sequence,
};
#[cfg(verus_keep_ghost)]
pub use automatic_verification::inventory::append_action_index;
#[cfg(verus_keep_ghost)]
pub use automatic_verification::migration::{
    adopted_policy_binding_error, checkout_migration_ready, policy_migration_decision,
    policy_state_conflict, policy_state_present, store_current_policy,
};
#[cfg(verus_keep_ghost)]
pub use automatic_verification::service::{
    maintenance_exclusions, maintenance_has_events, maintenance_retention_phase, next_enrollment,
    validate_activation_sequence,
};

#[cfg(not(verus_keep_ghost))]
pub use crate::automatic::producer::rolling::{
    clamp_rolling_deadline, rolling_disposal, rolling_keep,
};
#[cfg(not(verus_keep_ghost))]
pub use crate::automatic::producer::{
    ProducerDeclarations, ProducerLookupError, ProducerNamespaceMember, find_producer_declaration,
    producer_namespace_member, producer_namespace_retired, take_resolved_store,
    visit_producer_stores,
};
#[cfg(verus_keep_ghost)]
pub use automatic_verification::producer::rolling::{
    clamp_rolling_deadline, rolling_disposal, rolling_keep,
};
#[cfg(verus_keep_ghost)]
pub use automatic_verification::producer::{
    ProducerDeclarations, ProducerLookupError, ProducerNamespaceMember, find_producer_declaration,
    producer_namespace_member, producer_namespace_retired, producer_visit_prefix,
    take_resolved_store, visit_producer_stores,
};

#[cfg(not(verus_keep_ghost))]
pub use crate::automatic::retirement::{expired_original, retire_cache_history, retire_history};
#[cfg(not(verus_keep_ghost))]
pub use crate::recovery::{
    ImportedPolicyReceipt, imported_policy_receipt_valid, imported_policy_source_commit,
};
#[cfg(verus_keep_ghost)]
pub use automatic_verification::retirement::{
    expired_original, retire_cache_history, retire_history,
};
#[cfg(verus_keep_ghost)]
#[path = "recovery.rs"]
#[rustfmt::skip]
mod recovery_verification;
#[cfg(verus_keep_ghost)]
pub use recovery_verification::{
    ImportedPolicyReceipt, imported_policy_receipt_valid, imported_policy_source_commit,
    required_text,
};

#[cfg(not(verus_keep_ghost))]
pub use crate::successful_report_status;
#[cfg(verus_keep_ghost)]
#[path = "lib.rs"]
#[rustfmt::skip]
mod cli_status_owner;
#[cfg(verus_keep_ghost)]
pub use cli_status_owner::successful_report_status;

#[cfg(verus_keep_ghost)]
#[path = "log_retention.rs"]
pub mod log_retention;
#[cfg(not(verus_keep_ghost))]
pub use crate::log_retention;

verus! {

/// Classify one Git index row against the commit whose checkout is being cleaned.
pub fn index_entry_changed(stage: &str, mode: &str, oid: &str, expected: Option<(&str, &str)>)
    -> (out: Result<bool, &'static str>)
    ensures
        if stage@ == "0"@ {
            match expected {
                Some((tree_mode, tree_oid)) => out == Ok::<bool, &str>(
                    mode@ != tree_mode@ || oid@ != tree_oid@),
                None => out == Ok::<bool, &str>(true),
            }
        } else if stage@ == "1"@ || stage@ == "2"@ || stage@ == "3"@ {
            out == Ok::<bool, &str>(true)
        } else {
            out.is_err() && out->Err_0@ == "invalid Git index stage"@
        },
{
    if <&str>::exec_eq(stage, "0") {
        Ok(match expected {
            Some((tree_mode, tree_oid)) => !<&str>::exec_eq(mode, tree_mode)
                || !<&str>::exec_eq(oid, tree_oid),
            None => true,
        })
    } else if <&str>::exec_eq(stage, "1") || <&str>::exec_eq(stage, "2")
        || <&str>::exec_eq(stage, "3") {
        Ok(true)
    } else {
        Err("invalid Git index stage")
    }
}

pub enum ObservationAgeReply { Fresh, Stale, Diagnostic, Invalid }

pub fn observation_age_reply(exit: i32, stdout_empty: bool, message_empty: bool,
    stale_diagnostic: bool) -> (reply: ObservationAgeReply)
    ensures match reply {
        ObservationAgeReply::Fresh => exit == 0 && stdout_empty && message_empty,
        ObservationAgeReply::Stale => exit == 1 && stdout_empty && stale_diagnostic,
        ObservationAgeReply::Diagnostic => exit == 2 && stdout_empty && !message_empty,
        ObservationAgeReply::Invalid => !stdout_empty || (
            !(exit == 0 && message_empty) && !(exit == 1 && stale_diagnostic)
            && !(exit == 2 && !message_empty)),
    },
{
    if stdout_empty && exit == 0 && message_empty { ObservationAgeReply::Fresh }
    else if stdout_empty && exit == 1 && stale_diagnostic { ObservationAgeReply::Stale }
    else if stdout_empty && exit == 2 && !message_empty { ObservationAgeReply::Diagnostic }
    else { ObservationAgeReply::Invalid }
}

pub fn reuse_cached_observation(cached_fresh: bool, matching_context: bool, current_fresh: bool)
    -> (reuse: bool)
    ensures reuse == (cached_fresh && matching_context && current_fresh),
{ cached_fresh && matching_context && current_fresh }

pub fn log_owner_can_report(exit: i32, stderr_empty: bool) -> (valid: bool)
    ensures valid <==> (stderr_empty && (exit == 0 || exit == 1)),
{
    stderr_empty && (exit == 0 || exit == 1)
}

pub fn accept_log_report<T>(exit: i32, report: Option<T>, has_errors: bool)
    -> (result: Result<Option<T>, ()>)
    ensures
        result.is_ok() <==> ((!has_errors && exit == 0)
            || (has_errors && report.is_some() && exit == 1)),
        result.is_ok() ==> result == Ok::<Option<T>, ()>(report),
{
    if (!has_errors && exit == 0) || (has_errors && report.is_some() && exit == 1) {
        Ok(report)
    } else {
        Err(())
    }
}


/// 普通消费交接不改变观察、导入、迁移或部署的现有资格边界。
pub fn ordinary_admission(operation: &str) -> (selected: bool)
    ensures selected <==> (operation@ == "use"@ || operation@ == "check"@
        || operation@ == "bootstrap"@),
{
    <&str>::exec_eq(operation, "use") || <&str>::exec_eq(operation, "check")
        || <&str>::exec_eq(operation, "bootstrap")
}

/// An address left by failed admission has no checkout policy consumer.
pub fn enrolled_policy_required(entries: usize) -> (required: bool)
    ensures required <==> entries > 0,
{
    entries > 0
}

#[cfg_attr(not(verus_keep_ghost), derive(Clone, Debug, PartialEq, Eq, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(rename_all = "kebab-case"))]
pub enum Disposition {
    Dispose,
    FinishDispose,
    Retain,
    EvidenceRetain,
}

pub fn select_artifact(disposition: &Disposition, release_reusable: bool,
    release_evidence: bool) -> (selected: bool)
    ensures selected <==> match *disposition {
        Disposition::Dispose => true,
        Disposition::FinishDispose => release_reusable,
        Disposition::Retain => false,
        Disposition::EvidenceRetain => release_evidence,
    },
{
    match disposition {
        Disposition::Dispose => true,
        Disposition::FinishDispose => release_reusable,
        Disposition::Retain => false,
        Disposition::EvidenceRetain => release_evidence,
    }
}

#[cfg_attr(not(verus_keep_ghost), derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(rename_all = "kebab-case"))]
pub enum CleanupStage {
    Pending,
    InProgress,
    WaitingForConsumers,
    Retained,
    Failed,
    CachesReleased,
    Disposed,
}


/// History belongs to a finished checkout after all local responsibilities leave.
pub fn terminal_history_ready(uses: usize, pending_report: bool, cache_only: bool,
    artifacts_only: bool, cache_pending: Option<bool>) -> (ready: bool)
    ensures ready == (uses == 0 && !pending_report && !cache_only && !artifacts_only
        && cache_pending == Some(false)),
{
    uses == 0 && !pending_report && !cache_only && !artifacts_only
        && matches!(cache_pending, Some(false))
}

/// Actual archive receipts can carry inline terminal facts or the older original.
pub fn terminal_history_handoff(inline: Option<bool>, receipt_matches: bool,
    original_matches: bool) -> (bound: bool)
    ensures bound == (match inline {
        Some(eq) => eq || receipt_matches,
        None => receipt_matches && original_matches,
    }),
{
    match inline {
        Some(eq) => eq || receipt_matches,
        None => receipt_matches && original_matches,
    }
}

/// The recorded progress and the producer's reported effect must both establish disposal.
pub fn terminal_history_result(handoff: bool, same_checkout: bool, same_head: bool,
    disposed_progress: bool, same_receipt: bool, status: &str, removal: &str) -> (verified: bool)
    ensures verified == (handoff && same_checkout && same_head && disposed_progress && same_receipt
        && cleanup_stage(false,status@,removal@) == CleanupStage::Disposed),
{
    handoff && same_checkout && same_head && disposed_progress && same_receipt
        && matches!(cleanup_result(false, status, removal), CleanupStage::Disposed)
}

pub struct CacheHistoryBinding {
    pub receipt_matches: bool,
    pub result_checkout_matches: bool,
    pub result_generation_matches: bool,
    pub result_intent_matches: bool,
    pub intent_checkout_matches: bool,
    pub intent_attachment_matches: bool,
    pub intent_generation_matches: bool,
    pub owner_matches: bool,
}

/// Cache rotation consumes both the original result and the intent for its generation.
pub fn cache_history_result(binding: &CacheHistoryBinding, status: &str) -> (verified: bool)
    ensures verified == (status@ == "cleaned"@ && binding.receipt_matches
        && binding.result_checkout_matches && binding.result_generation_matches
        && binding.result_intent_matches && binding.intent_checkout_matches
        && binding.intent_attachment_matches && binding.intent_generation_matches && binding.owner_matches),
{
    <&str>::exec_eq(status,"cleaned") && binding.receipt_matches && binding.result_checkout_matches
        && binding.result_generation_matches && binding.result_intent_matches
        && binding.intent_checkout_matches && binding.intent_attachment_matches
        && binding.intent_generation_matches && binding.owner_matches
}

pub open spec fn cleanup_stage(cache: bool, status: Seq<char>, removal: Seq<char>) -> CleanupStage {
    if status != "cleaned"@ { CleanupStage::Failed }
    else if cache { CleanupStage::CachesReleased }
    else if removal == "verified-absent"@ || removal == "already-absent"@ {
        CleanupStage::Disposed
    } else { CleanupStage::Retained }
}

pub fn cleanup_result(cache: bool, status: &str, removal: &str) -> (stage: CleanupStage)
    ensures stage == cleanup_stage(cache, status@, removal@),
{
    if !<&str>::exec_eq(status, "cleaned") { CleanupStage::Failed }
    else if cache { CleanupStage::CachesReleased }
    else if <&str>::exec_eq(removal, "verified-absent") || <&str>::exec_eq(removal, "already-absent") {
        CleanupStage::Disposed
    } else { CleanupStage::Retained }
}

pub struct CleanupOutcome {
    pub cache: bool,
    pub cleaned: bool,
    pub stage: CleanupStage,
}

pub open spec fn cleanup_field(value: Option<&str>) -> Seq<char> {
    match value { Some(text) => text@, None => ""@ }
}

/// 原 JSON 观察只投影字段；缓存资格、成功与终态优先级由同一证明根决定。
pub fn observed_cleanup_result(cache: bool, enrollment_status: &str,
    age_outcome: Option<&str>, status: Option<&str>, removal: Option<&str>)
    -> (out: CleanupOutcome)
    ensures out.cache == (cleanup_field(age_outcome) == "reusable-source-preserved"@
            || cleanup_field(age_outcome) == "no-disposable-output-retained-checkout"@
            || (cache && cleanup_field(age_outcome)
                != "expired-checkout-verified-absent-source-branches-retained"@)),
        out.cleaned == (cleanup_field(status) == "cleaned"@),
        out.stage == cleanup_stage(out.cache && enrollment_status@ != "terminal"@,
            cleanup_field(status), cleanup_field(removal)),
{
    let age = match age_outcome { Some(value) => value, None => "" };
    let cache_outcome = <&str>::exec_eq(age, "reusable-source-preserved")
        || <&str>::exec_eq(age, "no-disposable-output-retained-checkout")
        || (cache && !<&str>::exec_eq(age, "expired-checkout-verified-absent-source-branches-retained"));
    let status = match status { Some(value) => value, None => "" };
    let removal = match removal { Some(value) => value, None => "" };
    let cleaned = <&str>::exec_eq(status, "cleaned");
    let stage = cleanup_result(cache_outcome && !<&str>::exec_eq(enrollment_status, "terminal"),
        status, removal);
    CleanupOutcome { cache: cache_outcome, cleaned, stage }
}

pub fn active_cache(status: &str, pending: bool) -> (active: bool)
    ensures active == (status@ == "active"@ && pending),
{
    <&str>::exec_eq(status, "active") && pending
}

pub fn select_cleanup(status: &str, cache_pending: bool, maintenance: bool, artifacts_only: bool) -> (selected: bool)
    ensures selected <==> (status@ == "terminal"@
        || (status@ == "active"@ && cache_pending)
        || (status@ == "retained"@ && maintenance && !artifacts_only)),
{
    <&str>::exec_eq(status, "terminal")
        || active_cache(status, cache_pending)
        || (<&str>::exec_eq(status, "retained") && maintenance && !artifacts_only)
}

/// Age admits a visited enrollment to the existing ownership and custody checks.
pub fn select_drain(status: &str, cache_pending: bool, maintenance: bool,
    artifacts_only: bool, age_due: bool) -> (selected: bool)
    ensures selected <==> (age_due || status@ == "terminal"@
        || (status@ == "active"@ && cache_pending)
        || (status@ == "retained"@ && maintenance && !artifacts_only)),
{
    age_due || select_cleanup(status, cache_pending, maintenance, artifacts_only)
}


pub enum DrainTarget { UnknownOwner, ExcludedPath, AcquireLease, Unowned }

/// An excluded checkout can only release an active pending cache in cache-only mode.
pub fn drain_target(kernel: bool, owned: bool, excluded: bool, active_cache: bool, cache_only: bool)
    -> (target: DrainTarget)
    ensures target == (if kernel && !owned { DrainTarget::UnknownOwner }
        else if excluded && !(active_cache && cache_only) { DrainTarget::ExcludedPath }
        else if owned { DrainTarget::AcquireLease } else { DrainTarget::Unowned }),
{
    if kernel && !owned { DrainTarget::UnknownOwner }
    else if excluded && !(active_cache && cache_only) { DrainTarget::ExcludedPath }
    else if owned { DrainTarget::AcquireLease }
    else { DrainTarget::Unowned }
}

pub enum DrainLease<L, E> { Held(L), Busy, Failed(E) }

/// Preserve the real capability or acquisition error from the kernel adapter.
pub fn drain_lease<L, E>(observation: Result<Option<L>, E>) -> (lease: DrainLease<L,E>)
    ensures lease == (match observation {
        Ok(Some(held)) => DrainLease::Held(held),
        Ok(None) => DrainLease::Busy,
        Err(error) => DrainLease::Failed(error),
    }),
{
    match observation {
        Ok(Some(held)) => DrainLease::Held(held),
        Ok(None) => DrainLease::Busy,
        Err(error) => DrainLease::Failed(error),
    }
}

pub fn drain_uses_released(remaining: usize) -> (released: bool)
    ensures released == (remaining == 0),
{
    remaining == 0
}

pub fn retain_evidence(disposal_authorized: bool, evidence_present: bool) -> (retain: bool)
    ensures retain <==> (!disposal_authorized && evidence_present),
{
    !disposal_authorized && evidence_present
}

pub fn accepts_enrollment_branch(kind: &str, registered: bool) -> (accepted: bool)
    ensures accepted <==> (kind@ == "import"@
        || (kind@ == "birth"@ && registered)),
{
    <&str>::exec_eq(kind, "import") || (<&str>::exec_eq(kind, "birth") && registered)
}

pub fn accepts_cleanup_branch(kind: &str, registered: bool) -> (accepted: bool)
    ensures accepted <==> (kind@ == "import"@ || registered),
{
    <&str>::exec_eq(kind, "import") || registered
}

#[cfg_attr(not(verus_keep_ghost), derive(Debug, PartialEq, Eq))]
pub enum AdoptedPolicySource {
    ImportedOriginal,
    Current,
    RetainedTarget,
}

pub struct AutomaticCleanupReceipt {
    pub schema_matches: bool,
    pub cleanup_operation: bool,
    pub automatic: bool,
    pub host_matches: bool,
    pub config_matches: bool,
    pub attempt_matches: bool,
    pub token_matches: bool,
}

/// The report owner, invocation and policy all identify the same automatic cleanup.
pub fn automatic_receipt_matches(r: &AutomaticCleanupReceipt) -> (matches: bool)
    ensures matches == (r.schema_matches && r.cleanup_operation && r.automatic && r.host_matches
        && r.config_matches && r.attempt_matches && r.token_matches),
{
    r.schema_matches && r.cleanup_operation && r.automatic && r.host_matches
        && r.config_matches && r.attempt_matches && r.token_matches
}

pub fn published_cleanup_receipt(r: &AutomaticCleanupReceipt, status: &str,
    inline: Option<bool>, original_receipt: bool) -> (matches: bool)
    ensures matches == (r.schema_matches && r.cleanup_operation && r.automatic && r.host_matches
        && r.config_matches && r.attempt_matches && r.token_matches
        && (status@ == "cleaned"@ || status@ == "failed"@)
        && (match inline { Some(equal) => equal, None => original_receipt })),
{
    automatic_receipt_matches(r) && (<&str>::exec_eq(status,"cleaned") || <&str>::exec_eq(status,"failed"))
        && (match inline { Some(equal) => equal, None => original_receipt })
}

pub fn artifacts_only_terminal(status: &str, artifacts_only: bool, disposing_evidence: bool) -> (applies: bool)
    ensures applies == (status@ == "disposed"@ && artifacts_only && !disposing_evidence),
{
    <&str>::exec_eq(status,"disposed") && artifacts_only && !disposing_evidence
}

pub fn preserved_cleanup_consumed_terminal(r: &AutomaticCleanupReceipt, released_and_sealed: bool,
    checkout_matches: bool, head_matches: bool, status: &str, removal: &str,
    same_receipt: bool, same_input: bool) -> (consumed: bool)
    ensures consumed == (r.schema_matches && r.cleanup_operation && r.automatic && r.host_matches
        && r.config_matches && r.attempt_matches && r.token_matches && released_and_sealed
        && checkout_matches && head_matches && status@ == "cleaned"@ && removal@ == "preserved"@
        && same_receipt && same_input),
{
    automatic_receipt_matches(r) && released_and_sealed && checkout_matches && head_matches
        && <&str>::exec_eq(status,"cleaned") && <&str>::exec_eq(removal,"preserved") && same_receipt && same_input
}

pub struct CleanupBinding<'a> {
    pub token: &'a str,
    pub report_path: &'a str,
    pub checkout: &'a str,
    pub branch: &'a str,
    pub head: &'a str,
    pub registry_digest: &'a str,
}

/// Match a published interrupted cleanup to the current terminal enrollment.
pub fn binds_interrupted_cleanup(operation: &str, original: &CleanupBinding, expected: &CleanupBinding)
    -> (bound: bool)
    ensures bound <==> (operation@ == "cleanup"@
        && original.token@ == expected.token@
        && original.report_path@ == expected.report_path@
        && original.checkout@ == expected.checkout@
        && original.branch@ == expected.branch@
        && original.head@ == expected.head@
        && original.registry_digest@ == expected.registry_digest@),
{
    <&str>::exec_eq(operation, "cleanup")
        && <&str>::exec_eq(original.token, expected.token)
        && <&str>::exec_eq(original.report_path, expected.report_path)
        && <&str>::exec_eq(original.checkout, expected.checkout)
        && <&str>::exec_eq(original.branch, expected.branch)
        && <&str>::exec_eq(original.head, expected.head)
        && <&str>::exec_eq(original.registry_digest, expected.registry_digest)
}

pub struct ManagedUseBinding<'a> {
    pub schema: &'a str,
    pub checkout: &'a str,
    pub operation: &'a str,
    pub report_path: &'a str,
    pub process_path: &'a str,
    pub joined: bool,
}

pub fn binds_use_intent(intent: &ManagedUseBinding, result: &ManagedUseBinding,
    report_path: &str, process_path: &str) -> (bound: bool)
    ensures bound <==> (intent.schema@ == "chrono-worktree-managed-use-intent/v2"@
        && result.schema@ == "chrono-worktree-managed-use-result/v1"@
        && !intent.joined && result.joined
        && intent.checkout@ != ""@ && intent.checkout@ == result.checkout@
        && intent.operation@ != ""@ && intent.operation@ == result.operation@
        && intent.report_path@ == report_path@ && result.report_path@ == report_path@
        && intent.process_path@ == process_path@ && result.process_path@ == process_path@),
{
    <&str>::exec_eq(intent.schema, "chrono-worktree-managed-use-intent/v2")
        && <&str>::exec_eq(result.schema, "chrono-worktree-managed-use-result/v1")
        && !intent.joined && result.joined
        && !<&str>::exec_eq(intent.checkout, "") && <&str>::exec_eq(intent.checkout, result.checkout)
        && !<&str>::exec_eq(intent.operation, "") && <&str>::exec_eq(intent.operation, result.operation)
        && <&str>::exec_eq(intent.report_path, report_path)
        && <&str>::exec_eq(result.report_path, report_path)
        && <&str>::exec_eq(intent.process_path, process_path)
        && <&str>::exec_eq(result.process_path, process_path)
}

pub fn needs_enrollment_report(cache_only: bool, pending_use: bool, pending_handoff: bool) -> (needed: bool)
    ensures needed == (!cache_only || pending_use || pending_handoff),
{
    !cache_only || pending_use || pending_handoff
}

pub fn imported_policy_required(kind: &str, migrated: bool) -> (required: bool)
    ensures required <==> (kind@ == "import"@ && !migrated),
{
    <&str>::exec_eq(kind, "import") && !migrated
}

pub fn restore_present_import(imported: bool, maintenance: bool,
    managed_originals: bool) -> (restore: bool)
    ensures restore <==> (imported && maintenance && managed_originals),
{
    imported && maintenance && managed_originals
}

pub fn adopted_policy_source(kind: &str, migrated: bool, head: &str, current_head: &str,
    digest: &str, current_digest: &str) -> (source: AdoptedPolicySource)
    ensures
        (source == AdoptedPolicySource::ImportedOriginal) <==>
            (kind@ == "import"@ && !migrated),
        (source == AdoptedPolicySource::Current) <==>
            (!(kind@ == "import"@ && !migrated)
                && head@ == current_head@ && digest@ == current_digest@),
        (source == AdoptedPolicySource::RetainedTarget) <==>
            (!(kind@ == "import"@ && !migrated)
                && (head@ != current_head@ || digest@ != current_digest@)),
{
    if imported_policy_required(kind, migrated) { AdoptedPolicySource::ImportedOriginal }
    else if <&str>::exec_eq(head, current_head) && <&str>::exec_eq(digest, current_digest) {
        AdoptedPolicySource::Current
    } else { AdoptedPolicySource::RetainedTarget }
}






}
