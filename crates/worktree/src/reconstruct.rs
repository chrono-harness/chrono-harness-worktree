use vstd::{contrib::exec_spec::ExecSpecEq, prelude::*};

#[cfg(not(verus_keep_ghost))]
use crate::document::sha256;
#[cfg(not(verus_keep_ghost))]
use crate::document::{decode, no_symlink_parents, relative_path};
#[cfg(not(verus_keep_ghost))]
use crate::facts;
#[cfg(not(verus_keep_ghost))]
use crate::{
    Start,
    start::{self, Runner, paths},
};
#[cfg(not(verus_keep_ghost))]
use serde::Deserialize;
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json as value};
#[cfg(not(verus_keep_ghost))]
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    schema: String,
    base: String,
    candidate: String,
    changes: Vec<Change>,
}
#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Change {
    path: String,
    action: Action,
    reason: Option<String>,
}
verus! {
#[cfg_attr(not(verus_keep_ghost), derive(Clone, Copy, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(rename_all = "lowercase"))]
pub(crate) enum Action { Carry, Retire }

pub(crate) fn valid_choice(fresh: bool, action: &Action, trimmed_reason: Option<&str>)
    -> (valid: bool)
    ensures valid == (fresh && (action is Carry || (trimmed_reason is Some
        && trimmed_reason->Some_0@.len() > 0))),
{
    fresh && match action {
        Action::Carry => true,
        Action::Retire => match trimmed_reason {
            Some(reason) => !reason.is_empty(),
            None => false,
        },
    }
}

pub(crate) enum Selection { Done, Mismatch, Carry(String), Retire }

/// Both iterators yield unique paths in the same string order.
pub(crate) fn select_path(changed: Option<String>, declared: Option<(String, Action)>)
    -> (selection: Selection)
    ensures
        (selection is Done) == (changed is None && declared is None),
        (selection is Mismatch) == (changed.is_some() != declared.is_some()
            || (changed is Some && declared is Some && changed->Some_0@ != declared->Some_0.0@)),
        (selection is Retire) == (changed is Some && declared is Some
            && changed->Some_0@ == declared->Some_0.0@ && declared->Some_0.1 is Retire),
        (selection is Carry) == (changed is Some && declared is Some
            && changed->Some_0@ == declared->Some_0.0@ && declared->Some_0.1 is Carry),
        selection is Carry ==> selection->Carry_0@ == changed->Some_0@,
{
    match (changed, declared) {
        (None, None) => Selection::Done,
        (Some(path), Some((choice, action))) if <&str>::exec_eq(path.as_str(), choice.as_str()) => {
            match action {
                Action::Carry => Selection::Carry(path),
                Action::Retire => Selection::Retire,
            }
        },
        _ => Selection::Mismatch,
    }
}
}

#[cfg(not(verus_keep_ghost))]
pub(crate) struct Input {
    path: String,
    bytes: Vec<u8>,
    base: String,
    candidate: String,
    choices: BTreeMap<String, Action>,
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn read(root: &Path, path: &str) -> Result<Input, String> {
    relative_path(path)?;
    if !path.starts_with(".chrono-harness/state/") {
        return Err("reconstruction plan must be under .chrono-harness/state".into());
    }
    let bytes = fs::read(no_symlink_parents(root, path)?).map_err(|e| e.to_string())?;
    let plan: Plan = decode(&bytes)?;
    if plan.schema != "chrono-worktree-reconstruction/v1" {
        return Err("unsupported reconstruction plan schema".into());
    }
    facts::full_oid(&plan.base)?;
    facts::full_oid(&plan.candidate)?;
    let mut choices = BTreeMap::new();
    for change in plan.changes {
        relative_path(&change.path)?;
        let fresh = choices.insert(change.path, change.action).is_none();
        if !valid_choice(
            fresh,
            &change.action,
            change.reason.as_deref().map(str::trim),
        ) {
            return Err("duplicate reconstruction path or retirement without a reason".into());
        }
    }
    Ok(Input {
        path: path.into(),
        bytes,
        base: plan.base,
        candidate: plan.candidate,
        choices,
    })
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn execute(
    r: &mut Runner,
    o: &Start,
    config_bytes: &[u8],
    token: &str,
    report: &mut Value,
    input: Input,
) -> Result<(), String> {
    report["reconstruction"] = value!({"path":input.path,"sha256":sha256(&input.bytes),"input_bytes":input.bytes,"source_base":input.base,"source_candidate":input.candidate});
    if r.oid(&o.root, "HEAD")? != input.candidate {
        return Err("reconstruction candidate differs from source HEAD".into());
    }
    for revision in [&input.base, &input.candidate] {
        if r.oid(&o.root, &format!("{revision}^{{commit}}"))? != *revision {
            return Err("reconstruction endpoint is not a commit".into());
        }
    }
    let ancestry = r.command(
        &o.root,
        &["merge-base", "--is-ancestor", &input.base, &input.candidate],
    )?;
    if ancestry.exit_code != 0 {
        return Err(format!(
            "reconstruction base ancestry check exited {}",
            ancestry.exit_code
        ));
    }
    let config_path = r.config.host_config.clone();
    let (registrations, _) = start::registrations(r, &o.root, &input.candidate, &config_path)?;
    if !registrations
        .client
        .nonartifact_paths(registrations.config(), std::slice::from_ref(&input.path))?
        .is_empty()
    {
        return Err("reconstruction plan lacks a registered artifact owner".into());
    }
    start::cleanliness(r, &o.root, registrations.config())?;
    let changed = paths(r.git(
        &o.root,
        &[
            "diff",
            "--name-only",
            "--no-renames",
            "--no-ext-diff",
            "--no-textconv",
            "-z",
            &input.base,
            &input.candidate,
            "--",
        ],
    )?)?;
    let mut changed = changed.into_iter();
    let mut declared = input.choices.into_iter();
    let mut carried = BTreeSet::new();
    loop {
        match select_path(changed.next(), declared.next()) {
            Selection::Done => break,
            Selection::Mismatch => {
                return Err(
                    "reconstruction choices must cover exactly the original DELTA paths".into(),
                );
            }
            Selection::Carry(path) => {
                carried.insert(path);
            }
            Selection::Retire => {}
        }
    }
    let patch = if carried.is_empty() {
        vec![]
    } else {
        let mut argv = vec![
            "--literal-pathspecs",
            "diff",
            "--binary",
            "--full-index",
            "--no-renames",
            "--no-ext-diff",
            "--no-textconv",
            "--src-prefix=a/",
            "--dst-prefix=b/",
            &input.base,
            &input.candidate,
            "--",
        ];
        argv.extend(carried.iter().map(String::as_str));
        r.git(&o.root, &argv)?
    };
    report["reconstruction"]["patch_sha256"] = value!(sha256(&patch));
    report["reconstruction"]["carried_paths"] = value!(carried);
    if r.oid(&o.root, "HEAD")? != input.candidate {
        return Err("source HEAD changed during reconstruction preparation".into());
    }
    // Keep the original creation lock until the staged result is verified.
    start::execute(r, o, config_bytes, token, report, true)?;
    let target = report["destination"]
        .as_str()
        .ok_or("missing destination")?
        .to_string();
    let branch = report["branch_ref"]
        .as_str()
        .ok_or("missing branch")?
        .to_string();
    let base = report["base"]
        .as_str()
        .ok_or("missing fetched base")?
        .to_string();
    report["context"]["candidate"] = Value::Null;
    let target_path = Path::new(&target);
    if !patch.is_empty() {
        let applied = r.input(
            target_path,
            &["apply", "--3way", "--index", "--binary", "-"],
            &patch,
        )?;
        if applied.exit_code != 0 {
            return Err(format!(
                "reconstruction apply exited {}; preserve worktree and lock for AI reconciliation",
                applied.exit_code
            ));
        }
    }
    start::ensure_identity(r, &o.root, target_path, &branch, &base, token)?;
    let tree = r.text(target_path, &["write-tree"])?.trim().to_string();
    facts::full_oid(&tree)?;
    start::exact_checkout(r, target_path, &tree)?;
    let staged = paths(r.git(
        target_path,
        &[
            "diff",
            "--cached",
            "--name-only",
            "--no-renames",
            "--no-ext-diff",
            "--no-textconv",
            "-z",
            &base,
            "--",
        ],
    )?)?;
    if !staged.is_subset(&carried) {
        return Err("reconstruction changed an undeclared path; preserve worktree and lock".into());
    }
    if r.oid(&o.root, "HEAD")? != input.candidate {
        return Err("source HEAD changed during reconstruction; preserve both worktrees".into());
    }
    start::cleanliness(r, &o.root, registrations.config())?;
    report["reconstruction"]["index_tree"] = value!(tree);
    report["reconstruction"]["staged_paths"] = value!(staged);
    report["recovery"] = value!(
        "Old work is preserved. Review the staged reconstruction, reconcile semantics, commit, then run the registered canonical check with new evidence."
    );
    start::unlock(r, &o.root, &target, &format!("refs/heads/{branch}"), &base)
}
