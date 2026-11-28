//! Automatic local endpoint/context production; start/reconstruct own branch birth.
#[cfg(verus_keep_ghost)]
use super::facts;
use vstd::contrib::exec_spec::ExecSpecEq;
#[cfg(not(verus_keep_ghost))]
use vstd::prelude::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub(crate) fn scoped_check_profile(schema: Option<&str>) -> (scoped: bool)
    ensures scoped == (facts::names(schema, "chrono-ci-check/v1"@)
        || facts::names(schema, "chrono-ci-check/v2"@) || facts::names(schema, "chrono-ci-check/v3"@)),
{
    facts::identity_text(schema, "chrono-ci-check/v1")
        || facts::identity_text(schema, "chrono-ci-check/v2")
        || facts::identity_text(schema, "chrono-ci-check/v3")
}

pub(crate) fn full_check_origin(
    schema: Option<&str>, destination_matches: bool, role: Option<&str>,
) -> (valid: bool)
    ensures valid == (facts::names(schema, "chrono-worktree-origin/v1"@) && destination_matches
        && (facts::names(role, "integration"@) || facts::names(role, "delivery"@))),
{
    facts::identity_text(schema, "chrono-worktree-origin/v1") && destination_matches
        && (facts::identity_text(role, "integration") || facts::identity_text(role, "delivery"))
}

pub(crate) fn full_check_birth(
    status: Option<&str>, destination_matches: bool, fork_matches_base: bool, processes_array: bool,
) -> (valid: bool)
    ensures valid == ((facts::names(status, "created"@) || facts::names(status, "reconstructed"@))
        && destination_matches && fork_matches_base && processes_array),
{
    (facts::identity_text(status, "created") || facts::identity_text(status, "reconstructed"))
        && destination_matches && fork_matches_base && processes_array
}

pub(crate) fn context_originals<'a>(
    context: Option<&'a str>, origin: Option<&'a str>, birth: Option<&'a str>,
) -> (paths: [Option<&'a str>; 3])
    ensures paths@ == seq![context, origin, birth],
{
    [context, origin, birth]
}

pub fn accepts_input_entry(expected: &str, observed: &str, regular: bool) -> (valid: bool)
    ensures valid == (regular && expected@ == observed@),
{
    regular && <&str>::exec_eq(expected, observed)
}

pub open spec fn usable_input_component(value: &str) -> bool {
    value@ != ""@ && value@ != "."@ && value@ != ".."@ && value@ != ".git"@
}

/// The caller passes the path split at every slash, including empty components.
pub fn dedicated_input_path(parts: &[&str]) -> (valid: bool)
    ensures valid == (parts.len() >= 4 && parts@[0]@ == ".chrono-harness"@
        && parts@[1]@ == "state"@
        && forall|i: int| 2 <= i < parts.len() ==> usable_input_component(parts@[i])),
{
    if parts.len() < 4 || !<&str>::exec_eq(parts[0], ".chrono-harness")
        || !<&str>::exec_eq(parts[1], "state") { return false; }
    let mut i = 2;
    while i < parts.len()
        invariant 2 <= i <= parts.len(),
            forall|j: int| 2 <= j < i ==> usable_input_component(parts@[j]),
        decreases parts.len() - i,
    {
        let part = parts[i];
        if <&str>::exec_eq(part, "") || <&str>::exec_eq(part, ".")
            || <&str>::exec_eq(part, "..") || <&str>::exec_eq(part, ".git") {
            return false;
        }
        i += 1;
    }
    true
}

pub fn unchanged_input(first: Option<(&str, u64)>, last: Option<(&str, u64)>) -> (same: bool)
    ensures same == match (first, last) {
        (Some((a, n)), Some((b, m))) => a@ == b@ && n == m,
        (None, None) => true,
        _ => false,
    },
{
    match (first, last) {
        (Some((a, n)), Some((b, m))) => <&str>::exec_eq(a, b) && n == m,
        (None, None) => true,
        _ => false,
    }
}

pub fn full_paths_distinct(origin: &str, context: &str, manifest: &str) -> (valid: bool)
    ensures valid == (origin@ != context@ && origin@ != manifest@ && context@ != manifest@),
{
    !<&str>::exec_eq(origin, context) && !<&str>::exec_eq(origin, manifest) && !<&str>::exec_eq(context, manifest)
}
}

#[cfg(not(verus_keep_ghost))]
use crate::{Config, configuration, start};
#[cfg(not(verus_keep_ghost))]
pub(crate) mod observation_age;
#[cfg(not(verus_keep_ghost))]
use crate::document as wire;
#[cfg(not(verus_keep_ghost))]
use crate::document::sha256;
#[cfg(not(verus_keep_ghost))]
use crate::facts;
#[cfg(not(verus_keep_ghost))]
use crate::{
    document::{decode, file_identity, no_symlink_parents},
    prepared::{self, Context, InputRequest, PreparedCheck, Scope, Selection, artifact_path},
};
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize};
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::Path,
};
#[cfg(not(verus_keep_ghost))]
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

/// 完整检查登记创建原件和上下文。范围检查只登记收集清单。
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged, deny_unknown_fields)]
#[cfg(not(verus_keep_ghost))]
pub enum Policy {
    Full(FullPolicy),
    Scoped {
        collection_manifest: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        optional_input_file: Option<String>,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[cfg(not(verus_keep_ghost))]
pub struct FullPolicy {
    #[serde(default = "default_observation_age_program")]
    pub observation_age_program: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub optional_input_file: Option<String>,
    pub origin_path: String,
    pub context_path: String,
    pub collection_manifest: String,
    pub roles: BTreeMap<String, String>,
}

#[cfg(not(verus_keep_ghost))]
fn default_observation_age_program() -> String {
    "chrono-judge-workflow".into()
}

#[cfg(not(verus_keep_ghost))]
impl Policy {
    fn optional_input_file(&self) -> Option<&str> {
        match self {
            Self::Full(policy) => policy.optional_input_file.as_deref(),
            Self::Scoped {
                optional_input_file,
                ..
            } => optional_input_file.as_deref(),
        }
    }

    fn collection_manifest(&self) -> &str {
        match self {
            Self::Full(full) => &full.collection_manifest,
            Self::Scoped {
                collection_manifest,
                ..
            } => collection_manifest,
        }
    }
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn validate(policy: &Policy) -> Result<(), String> {
    artifact_path(policy.collection_manifest())?;
    if let Some(path) = policy.optional_input_file() {
        artifact_path(path)?;
        let components: Vec<_> = path.split('/').collect();
        if !dedicated_input_path(&components) {
            return Err("single-file input must name a file in a dedicated state directory".into());
        }
    }
    let Policy::Full(p) = policy else {
        return Ok(());
    };
    artifact_path(&p.origin_path)?;
    artifact_path(&p.context_path)?;
    if !full_paths_distinct(&p.origin_path, &p.context_path, &p.collection_manifest)
        || p.roles.is_empty()
        || p.roles.iter().any(|(k, v)| {
            !matches!(k.as_str(), "feature" | "integration")
                || !matches!(v.as_str(), "integration" | "delivery")
        })
    {
        return Err("invalid local check origin/context/role binding".into());
    }
    Ok(())
}
/// A declared subdirectory in the existing state area may contain only its optional data file.
#[cfg(not(verus_keep_ghost))]
fn observe_input_directory(root: &Path, path: &str) -> Result<Option<(String, u64)>, String> {
    let file = no_symlink_parents(root, path)?;
    let parent = file.parent().ok_or("missing declared input directory")?;
    let name = file
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("invalid declared filename")?;
    let entries = match fs::read_dir(parent) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("read {}: {e}", parent.display())),
    };
    let mut observed = None;
    for item in entries {
        let item = item.map_err(|e| e.to_string())?;
        let actual = item.file_name();
        let kind = item.file_type().map_err(|e| e.to_string())?;
        if !accepts_input_entry(name, actual.to_str().unwrap_or(""), kind.is_file()) {
            return Err(format!(
                "unexpected installation data entry: {}",
                item.path().display()
            ));
        }
        observed = Some(file_identity(&file)?);
    }
    Ok(observed)
}

/// Publish the final original producer report in the destination, then its exact association.
#[cfg(not(verus_keep_ghost))]
pub(crate) fn publish_origin(source: &Path, report: &Value, p: &FullPolicy) -> Result<(), String> {
    let root = Path::new(
        report["destination"]
            .as_str()
            .ok_or("origin destination missing")?,
    );
    let role = p
        .roles
        .get(
            report["creation_kind"]
                .as_str()
                .ok_or("origin creation kind missing")?,
        )
        .ok_or("creation kind lacks explicit run-role mapping")?;
    let original = report["report_path"]
        .as_str()
        .ok_or("origin original report missing")?;
    let bytes = fs::read(no_symlink_parents(source, original)?).map_err(|e| e.to_string())?;
    if decode::<Value>(&bytes)? != *report {
        return Err("origin report differs from actual finalized producer output".into());
    }
    let path = format!(".chrono-harness/state/origins/{}.json", sha256(&bytes));
    publish(root, &path, &bytes, false)?;
    let origin = json!({"schema":"chrono-worktree-origin/v1","destination":root,"birth_report":path,"birth_sha256":sha256(&bytes),"run_kind":role,"integration_evidence":null,"retained_inputs":null});
    publish(
        root,
        &p.origin_path,
        &serde_json::to_vec(&origin).map_err(|e| e.to_string())?,
        false,
    )
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn publish(root: &Path, path: &str, bytes: &[u8], replace: bool) -> Result<(), String> {
    let target = no_symlink_parents(root, path)?;
    if target.exists() && !replace {
        if fs::read(&target).map_err(|e| e.to_string())? == bytes {
            return Ok(());
        }
        return Err(format!("producer output collision: {path}"));
    }
    let parent = target.parent().ok_or("output parent")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut f = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    f.write_all(bytes).map_err(|e| e.to_string())?;
    if replace {
        f.persist(&target).map_err(|e| e.to_string())?;
    } else {
        f.persist_noclobber(&target).map_err(|e| e.to_string())?;
    }
    Ok(())
}
#[cfg(not(verus_keep_ghost))]
fn produce(
    req: &InputRequest,
    path: &str,
    config: Config,
    bytes: &[u8],
) -> Result<PreparedCheck, String> {
    req.validate(&config)?;
    if config.host_config != req.host_config {
        return Err("local input source/host binding mismatch".into());
    }
    let _read_scope = crate::retention::read_scope(&req.host_root)?;
    let retention = crate::retention::Inventory::adopted(&req.host_root)?;
    let consumer = retention
        .as_ref()
        .map(|_| {
            crate::retention::unique_path(&req.host_root, "local-inputs-", "")
                .map(|path| path.file_name().unwrap().to_str().unwrap().to_string())
        })
        .transpose()?;
    let report = start::with_report_for_consumer(
        &req.host_root,
        path,
        config,
        bytes,
        "check-inputs",
        "prepared",
        consumer.as_deref(),
        |_, _| Ok((req.host_root.clone(), ())),
        |r, token, report, ()| {
            let root = &req.host_root;
            let observed = r.checkout_identity(root)?;
            let candidate = observed.head;
            let top = fs::canonicalize(&observed.top).map_err(|e| e.to_string())?;
            if top != *root {
                return Err("short check cwd must be the actual host root".into());
            }
            if r.blob(root, &candidate, path)? != bytes
                || sha256(&r.blob(root, &candidate, &req.profile)?) != req.profile_sha256
            {
                return Err("local input policy/profile differs from committed HEAD".into());
            }
            if let Some(p) = &req.initial_profile {
                if sha256(&r.blob(root, &candidate, &p.path)?) != p.sha256 {
                    return Err("local initial profile differs from committed HEAD".into());
                }
            }
            let (registrations, digest) =
                start::registrations(r, root, &candidate, &req.host_config)?;
            start::registered_policy(&registrations, path, &r.config.report_directory)?;
            let input = r
                .config
                .check_inputs
                .as_ref()
                .and_then(Policy::optional_input_file)
                .map(str::to_owned);
            let observe = || match input.as_deref() {
                Some(path) => observe_input_directory(root, path),
                None => Ok(None),
            };
            let initial_input = observe()?;
            if let Some(path) = &input {
                report["optional_input_file"] = json!({"path":path,"identity":initial_input});
            }
            let verify_input = || -> Result<(), String> {
                fn identity((hash, bytes): &(String, u64)) -> (&str, u64) {
                    (hash.as_str(), *bytes)
                }
                if !unchanged_input(
                    initial_input.as_ref().map(identity),
                    observe()?.as_ref().map(identity),
                ) {
                    return Err("declared input changed during local preparation".into());
                }
                Ok(())
            };
            start::cleanliness_at(r, root, &candidate, registrations.config())?;
            let parents =
                facts::commit_parents(&r.git(root, &["cat-file", "commit", &candidate])?)?;
            let initial = parents.is_empty();
            let selected_profile = req.profile_for(initial)?.to_string();
            report["parents"] = json!(parents);
            report["initial"] = json!(initial);
            report["profile"] = json!(selected_profile);
            if initial {
                // A physical root needs no invented or missing remote base.
                // The final HEAD/config checks still apply to this inventory path.
                if r.oid(root, "HEAD")? != candidate {
                    return Err("HEAD advanced during local preparation".into());
                }
                start::cleanliness_at(r, root, &candidate, registrations.config())?;
                verify_input()?;
                req.validate(&r.config)?;
                report["registry_digest"] = json!(digest);
                report["base"] = Value::Null;
                report["candidate"] = json!(candidate);
                report["check_context"] = Value::Null;
                report["scope"] = Value::Null;
                return Ok(());
            }
            let target_branch = registrations.workflow()["target_branch"]
                .as_str()
                .ok_or("local input target branch missing")?
                .to_string();
            let reference = format!("refs/heads/{target_branch}");
            r.git(root, &["check-ref-format", &reference])?;
            let remote = r.config.remote.clone();
            let fetched = format!("refs/chrono-harness/check/{token}");
            let spec = format!("{reference}:{fetched}");
            report["remote"] = json!(remote);
            report["target_ref"] = json!(reference);
            report["fetch_ref"] = json!(fetched);
            r.git(
                root,
                &[
                    "fetch",
                    "--no-tags",
                    "--no-write-fetch-head",
                    "--refmap=",
                    "--",
                    &remote,
                    &spec,
                ],
            )?;
            let base = r.oid(root, &fetched)?;
            r.oid(root, &format!("{base}^{{commit}}"))?;
            r.git(root, &["update-ref", "-d", &fetched, &base])?;
            report["fetch_ref_removed"] = json!(true);
            let profile: Value = decode(
                &fs::read(no_symlink_parents(root, &req.profile)?).map_err(|e| e.to_string())?,
            )?;
            let scoped = scoped_check_profile(profile["schema"].as_str());
            let scope = match &req.selection {
                Selection::All => None,
                Selection::Unit { unit } => Some(Scope::Unit { unit: unit.clone() }),
                Selection::Collect => Some(Scope::Collect {
                    manifest: r
                        .config
                        .check_inputs
                        .as_ref()
                        .ok_or("local collection manifest binding missing")?
                        .collection_manifest()
                        .to_owned(),
                }),
            };
            let context = if scoped {
                None
            } else {
                let policy = r
                    .config
                    .check_inputs
                    .as_ref()
                    .ok_or("full local check context/origin binding missing")?;
                let Policy::Full(p) = policy else {
                    return Err(
                        "full local check requires a registered branch origin and context".into(),
                    );
                };
                let p = p.clone();
                let origin_bytes =
                    fs::read(no_symlink_parents(root, &p.origin_path)?).map_err(|e| {
                        format!("missing full check origin receipt {}: {e}", p.origin_path)
                    })?;
                let origin: Value = decode(&origin_bytes)?;
                if !full_check_origin(
                    origin["schema"].as_str(),
                    origin["destination"] == json!(root),
                    origin["run_kind"].as_str(),
                ) {
                    return Err("invalid full check origin identity/role".into());
                }
                let birth_path = origin["birth_report"]
                    .as_str()
                    .ok_or("origin birth report reference missing")?;
                artifact_path(birth_path)?;
                let birth_bytes =
                    fs::read(no_symlink_parents(root, birth_path)?).map_err(|e| e.to_string())?;
                if !facts::identity_text(origin["birth_sha256"].as_str(), &sha256(&birth_bytes)) {
                    return Err("full check origin birth report drift".into());
                }
                let birth: Value = decode(&birth_bytes)?;
                if !full_check_birth(
                    birth["status"].as_str(),
                    birth["destination"] == json!(root),
                    birth["context"]["fork_point"] == birth["base"],
                    birth["processes"].is_array(),
                ) {
                    return Err("full check origin does not reference successful creation".into());
                }
                let branch = r
                    .text(root, &["symbolic-ref", "--short", "HEAD"])?
                    .trim()
                    .to_string();
                if !facts::identity_text(birth["branch_ref"].as_str(), &branch) {
                    return Err("full check branch differs from original birth association".into());
                }
                let fork = birth["context"]["fork_point"]
                    .as_str()
                    .ok_or("origin fork point missing")?;
                facts::full_oid(fork)?;
                r.oid(root, &format!("{fork}^{{commit}}"))?;
                let observed = OffsetDateTime::now_utc();
                let mut ctx = json!({"schema_version":2,"base":base,"candidate":candidate,"dev_tip":base,"branch_ref":branch,"fork_point":fork,"branch_started_at":birth["branch_started_at"],"observed_at":observed.format(&Rfc3339).map_err(|e|e.to_string())?,"operation":"validate.delta","run_kind":origin["run_kind"],"integration_evidence":origin["integration_evidence"]});
                if let Some(retained) = origin["retained_inputs"].as_str() {
                    artifact_path(retained)?;
                    file_identity(&no_symlink_parents(root, retained)?)?;
                    ctx["retained_inputs"] = json!(retained);
                }
                // Independent contributions and collection bind the same exact context.
                // Reuse only a matching current projection; endpoints/origin still come from this producer.
                if scope.is_some() {
                    let current = no_symlink_parents(root, &p.context_path)?;
                    match fs::read(current) {
                        Ok(raw) => {
                            let previous: Value = decode(&raw)?;
                            if !previous.is_object() || previous["schema_version"] != 2 {
                                return Err(
                                    "E_LOCAL_CONTEXT: cached context must be a schema2 object"
                                        .into(),
                                );
                            }
                            let fresh = |context: &Value| {
                                observation_age::check(
                                    context,
                                    registrations.workflow(),
                                    &p.observation_age_program,
                                    r.config.timeout_seconds,
                                    r.config.output_limit_bytes,
                                    &r.environment,
                                    |command, input| {
                                        let _custody = crate::retention::forward_scope()?;
                                        let result =
                                            crate::process::run_process(root, command, input)?;
                                        Ok((
                                            result.exit_code,
                                            result.stdout_bytes,
                                            result.stderr_bytes,
                                        ))
                                    },
                                )
                            };
                            let cached_fresh =
                                fresh(&previous).map_err(|e| format!("E_LOCAL_CONTEXT: {e}"))?;
                            let mut comparison = previous.clone();
                            comparison["observed_at"] = ctx["observed_at"].clone();
                            let matching_context = comparison == ctx;
                            let current_fresh = matching_context && fresh(&ctx)?;
                            if crate::verified_lifecycle::reuse_cached_observation(
                                cached_fresh,
                                matching_context,
                                current_fresh,
                            ) {
                                ctx = previous;
                            }
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                        Err(e) => return Err(e.to_string()),
                    }
                }
                // Keep the exact producer context immutable; the configured path is its current projection.
                let retained = prepared::retain(root, "local-context", &ctx)?;
                let retained_raw =
                    fs::read(no_symlink_parents(root, &retained)?).map_err(|e| e.to_string())?;
                publish(root, &p.context_path, &retained_raw, true)?;
                report["origin"] = json!({"path":p.origin_path,"sha256":sha256(&origin_bytes),"birth_report":birth_path,"birth_sha256":sha256(&birth_bytes)});
                report["current_observation"] = json!({"context_digest":wire::digest(&ctx)?,"unix_timestamp_nanos":observed.unix_timestamp_nanos().to_string()});
                Some(Context {
                    path: retained,
                    sha256: sha256(&retained_raw),
                    semantic_digest: wire::digest(&ctx)?,
                    raw: retained_raw,
                })
            };
            if !facts::identity_text(Some(&r.oid(root, "HEAD")?), &candidate) {
                return Err("HEAD advanced during local preparation".into());
            }
            start::cleanliness_at(r, root, &candidate, registrations.config())?;
            verify_input()?;
            req.validate(&r.config)?;
            report["registry_digest"] = json!(digest);
            report["base"] = json!(base);
            report["candidate"] = json!(candidate);
            report["check_context"] = json!(context);
            report["scope"] = json!(scope);
            Ok(())
        },
    )?;
    if !facts::identity_text(report["status"].as_str(), "prepared") {
        return Err(serde_json::to_string(&report).map_err(|e| e.to_string())?);
    }
    let mut originals = vec![prepared::original(
        &req.host_root,
        report["report_path"].as_str().ok_or("report path")?,
    )?];
    for path in context_originals(
        report["check_context"]["path"].as_str(),
        report["origin"]["path"].as_str(),
        report["origin"]["birth_report"].as_str(),
    )
    .into_iter()
    .flatten()
    {
        originals.push(prepared::original(&req.host_root, path)?);
    }
    let mut evidence = json!({"report_path":report["report_path"],"report_sha256":file_identity(&no_symlink_parents(&req.host_root,report["report_path"].as_str().ok_or("report path")?)?)?.0,"origin":report["origin"],"remote":report["remote"],"target_ref":report["target_ref"]});
    if let Some(observation) = report.get("current_observation") {
        evidence["current_observation"] = observation.clone();
    }
    if let Some(consumer) = report.get("report_consumer") {
        evidence["report_consumer"] = consumer.clone();
    }
    if let Some(inventory) = retention {
        inventory.dependencies_paths(
            report["report_path"].as_str().ok_or("report path")?,
            &originals
                .iter()
                .skip(1)
                .map(|original| original.path.clone())
                .collect::<Vec<_>>(),
        )?;
    }
    Ok(PreparedCheck {
        schema: prepared::RESPONSE.into(),
        request_sha256: sha256(&serde_json::to_vec(req).map_err(|e| e.to_string())?),
        source: req.source.clone(),
        profile: report["profile"].as_str().ok_or("profile missing")?.into(),
        base: report["base"].as_str().map(str::to_owned),
        candidate: report["candidate"]
            .as_str()
            .ok_or("candidate missing")?
            .into(),
        initial: report["initial"].as_bool().ok_or("initial missing")?,
        context: serde_json::from_value(report["check_context"].clone())
            .map_err(|e| e.to_string())?,
        scope: serde_json::from_value(report["scope"].clone()).map_err(|e| e.to_string())?,
        originals,
        evidence,
    })
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn dispatch(args: &[String]) -> Result<String, String> {
    if args.len() != 3 || args[0] != "check-inputs" || args[1] != "--config" {
        return Err("check-inputs requires its registered worktree --config".into());
    }
    let mut raw = Vec::new();
    std::io::stdin()
        .take(64 * 1024 * 1024 + 1)
        .read_to_end(&mut raw)
        .map_err(|e| e.to_string())?;
    if raw.len() > 64 * 1024 * 1024 {
        return Err("check input request exceeds bound".into());
    }
    let req: InputRequest = decode(&raw)?;
    let (config, bytes) = configuration(&req.host_root, &args[2])?;
    let p = produce(&req, &args[2], config, &bytes)?;
    Ok(serde_json::to_string(&p).map_err(|e| e.to_string())? + "\n")
}
