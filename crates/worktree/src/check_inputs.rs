//! Automatic local endpoint/context production; start/reconstruct own branch birth.
use vstd::contrib::exec_spec::ExecSpecEq;
#[cfg(not(verus_keep_ghost))]
use vstd::prelude::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub fn accepts_input_entry(expected: &str, observed: &str, regular: bool) -> (valid: bool)
    ensures valid == (regular && expected@ == observed@),
{
    regular && <&str>::exec_eq(expected, observed)
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
        single_file_directory: Option<String>,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[cfg(not(verus_keep_ghost))]
pub struct FullPolicy {
    #[serde(default = "default_observation_age_program")]
    pub observation_age_program: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub single_file_directory: Option<String>,
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
    fn single_file_directory(&self) -> Option<&str> {
        match self {
            Self::Full(policy) => policy.single_file_directory.as_deref(),
            Self::Scoped {
                single_file_directory,
                ..
            } => single_file_directory.as_deref(),
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
    if let Some(path) = policy.single_file_directory() {
        artifact_path(path)?;
        if path
            .split('/')
            .any(|part| matches!(part, "" | "." | ".." | ".git"))
            || Path::new(path).parent() == Some(Path::new(".chrono-harness/state"))
        {
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
    if req.source != "local" || req.prepared.is_some() || config.host_config != req.host_config {
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
                .and_then(Policy::single_file_directory)
                .map(str::to_owned);
            let observe = || {
                input
                    .as_ref()
                    .map(|path| observe_input_directory(root, path))
                    .transpose()
            };
            let initial_input = observe()?;
            if let Some(path) = &input {
                report["single_file_directory"] = json!({"path":path,"file":initial_input});
            }
            let verify_input = || -> Result<(), String> {
                if observe()? != initial_input {
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
            let scoped = matches!(
                profile["schema"].as_str(),
                Some("chrono-ci-check/v1" | "chrono-ci-check/v2" | "chrono-ci-check/v3")
            );
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
                if origin["schema"] != "chrono-worktree-origin/v1"
                    || origin["destination"] != json!(root)
                    || !matches!(
                        origin["run_kind"].as_str(),
                        Some("integration" | "delivery")
                    )
                {
                    return Err("invalid full check origin identity/role".into());
                }
                let birth_path = origin["birth_report"]
                    .as_str()
                    .ok_or("origin birth report reference missing")?;
                artifact_path(birth_path)?;
                let birth_bytes =
                    fs::read(no_symlink_parents(root, birth_path)?).map_err(|e| e.to_string())?;
                if origin["birth_sha256"] != sha256(&birth_bytes) {
                    return Err("full check origin birth report drift".into());
                }
                let birth: Value = decode(&birth_bytes)?;
                if !matches!(birth["status"].as_str(), Some("created" | "reconstructed"))
                    || birth["destination"] != json!(root)
                    || birth["context"]["fork_point"] != birth["base"]
                    || !birth["processes"].is_array()
                {
                    return Err("full check origin does not reference successful creation".into());
                }
                let branch = r
                    .text(root, &["symbolic-ref", "--short", "HEAD"])?
                    .trim()
                    .to_string();
                if birth["branch_ref"] != branch {
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
            if r.oid(root, "HEAD")? != candidate {
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
    if report["status"] != "prepared" {
        return Err(serde_json::to_string(&report).map_err(|e| e.to_string())?);
    }
    let mut originals = vec![prepared::original(
        &req.host_root,
        report["report_path"].as_str().ok_or("report path")?,
    )?];
    if let Some(path) = report["check_context"]["path"].as_str() {
        originals.push(prepared::original(&req.host_root, path)?);
    }
    if let Some(path) = report["origin"]["path"].as_str() {
        originals.push(prepared::original(&req.host_root, path)?);
    }
    if let Some(path) = report["origin"]["birth_report"].as_str() {
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
