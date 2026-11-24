//! Explicit remote branch disposal with saved-work checks and an exact push lease.
use vstd::contrib::exec_spec::ExecSpecEq;
#[cfg(not(verus_keep_ghost))]
use vstd::prelude::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub open spec fn same_text(value: Option<&str>, expected: &str) -> bool {
    match value { Some(observed) => observed@ == expected@, None => false }
}

pub fn single_endpoint(first: Option<&str>, second: Option<&str>, expected: &str) -> (ok: bool)
    ensures ok == (same_text(first, expected) && second.is_none()),
{
    match first { Some(url) => <&str>::exec_eq(url, expected) && second.is_none(), None => false }
}

pub struct Advertised<'a> {
    pub branch: Option<&'a str>,
    pub target: Option<&'a str>,
}

/// The Git observation is UTF-8 and its object ID has already passed full_oid.
pub fn record_ref<'a>(seen: Advertised<'a>, oid: &'a str, name: &str, branch: &str, target: &str)
    -> (out: Result<Advertised<'a>, &'static str>)
    requires branch@ != target@,
    ensures match out {
        Ok(next) => (name@ == branch@ && seen.branch.is_none() && next.branch == Some(oid) && next.target == seen.target)
            || (name@ == target@ && seen.target.is_none() && next.target == Some(oid) && next.branch == seen.branch),
        Err(reason) => reason == "unexpected or duplicate remote ref observation" &&
            ((name@ != branch@ && name@ != target@) || (name@ == branch@ && seen.branch.is_some())
             || (name@ == target@ && seen.target.is_some())),
    },
{
    if <&str>::exec_eq(name, branch) && seen.branch.is_none() {
        Ok(Advertised { branch: Some(oid), target: seen.target })
    } else if <&str>::exec_eq(name, target) && seen.target.is_none() {
        Ok(Advertised { branch: seen.branch, target: Some(oid) })
    } else {
        Err("unexpected or duplicate remote ref observation")
    }
}

pub fn retained_branch<'a>(seen: Advertised<'a>, expected: &str)
    -> (out: Result<Option<&'a str>, &'static str>)
    ensures if same_text(seen.target, expected) {
        out == Ok::<Option<&str>, &'static str>(seen.branch)
    } else {
        out == Err::<Option<&str>, &'static str>("remote target does not match retained commit")
    },
{
    match seen.target {
        Some(oid) if <&str>::exec_eq(oid, expected) => Ok(seen.branch),
        _ => Err("remote target does not match retained commit"),
    }
}

pub fn already_absent(observed: Option<&str>, head: &str, retry: bool) -> (out: Result<bool, &'static str>)
    ensures if same_text(observed, head) { out == Ok::<bool, &'static str>(false) }
        else if observed.is_none() && retry { out == Ok::<bool, &'static str>(true) }
        else { out == Err::<bool, &'static str>("remote branch changed or absent without an explicit retry plan") },
{
    match observed {
        None if retry => Ok(true),
        Some(oid) if <&str>::exec_eq(oid, head) => Ok(false),
        _ => Err("remote branch changed or absent without an explicit retry plan"),
    }
}
}

#[cfg(not(verus_keep_ghost))]
pub fn read_advertisement(
    text: &str,
    branch: &str,
    target: &str,
    retained: &str,
) -> Result<Option<String>, String> {
    if branch == target {
        return Err("remote cleanup requires distinct branch and retained references".into());
    }
    let mut seen = Advertised {
        branch: None,
        target: None,
    };
    for line in text.lines() {
        let (oid, name) = line
            .split_once('\t')
            .ok_or("invalid remote ref observation")?;
        facts::full_oid(oid)?;
        seen = record_ref(seen, oid, name, branch, target)?;
    }
    Ok(retained_branch(seen, retained)?.map(str::to_owned))
}

#[cfg(not(verus_keep_ghost))]
use crate::facts;
#[cfg(not(verus_keep_ghost))]
use crate::registration::Registrations;
#[cfg(not(verus_keep_ghost))]
use crate::{
    maintenance::{Retention, saved_commit},
    start::Runner,
};
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::path::Path;

#[cfg(not(verus_keep_ghost))]
pub(crate) struct Cleanup {
    pub(crate) head: String,
    pub(crate) retained_commit: String,
    pub(crate) branch: String,
    pub(crate) expected_url: String,
    pub(crate) retained_ref: String,
    pub(crate) retention: Retention,
    pub(crate) allow_absent_ref: bool,
}

#[cfg(not(verus_keep_ghost))]
impl Cleanup {
    fn saved(&self, r: &mut Runner, root: &Path) -> Result<(), String> {
        // The remote target is bound by observed(); only the pinned commit is local.
        saved_commit(
            r,
            root,
            &self.head,
            &format!("{}^{{commit}}", self.retained_commit),
            &self.retained_commit,
            &self.retention,
        )
    }

    pub(crate) fn execute(
        &self,
        r: &mut Runner,
        root: &Path,
        registrations: &Registrations,
        report: &mut Value,
        stable: impl Fn() -> Result<(), String>,
    ) -> Result<(), String> {
        report["remote_branch_removed"] = json!(false);
        report["remote_branch_removal"] = json!("not-attempted");
        let branch_ref = format!("refs/heads/{}", self.branch);
        r.git(root, &["check-ref-format", &branch_ref])?;
        r.git(root, &["check-ref-format", &self.retained_ref])?;
        let workflow = registrations.workflow();
        let target = format!(
            "refs/heads/{}",
            workflow["target_branch"]
                .as_str()
                .ok_or("missing workflow target")?
        );
        if self.retained_ref != target
            || branch_ref == target
            || !["feature_prefix", "integration_prefix"].iter().any(|k| {
                workflow[k]
                    .as_str()
                    .is_some_and(|p| self.branch.starts_with(p))
            })
        {
            return Err("remote cleanup requires a registered work branch and the distinct configured target".into());
        }
        if self.expected_url.is_empty()
            || self.expected_url.starts_with('-')
            || self.expected_url.contains(['\0', '\n', '\r'])
        {
            return Err("invalid expected remote push URL".into());
        }
        report["remote"] = json!(r.config.remote);
        report["remote_url"] = json!(self.expected_url);
        report["branch_ref"] = json!(self.branch);
        report["head"] = json!(self.head);
        report["retained_ref"] = json!(self.retained_ref);
        report["retained_commit"] = json!(self.retained_commit);
        let remote = r.config.remote.clone();
        let selection = RemoteSelection {
            remote: &remote,
            endpoint: &self.expected_url,
            branch_ref: &branch_ref,
            retained_ref: &self.retained_ref,
            retained: &self.retained_commit,
            head: &self.head,
            retry_absent: self.allow_absent_ref,
        };
        selection.run(report, |check| match check {
            RemoteCheck::Git(argv) => r.git(root, argv),
            RemoteCheck::Saved => self.saved(r, root).map(|()| Vec::new()),
            RemoteCheck::Stable => stable().map(|()| Vec::new()),
        })?;

        Ok(())
    }
}

/// Git dispatch and the caller's existing saved-work and plan checks.
#[cfg(not(verus_keep_ghost))]
pub enum RemoteCheck<'a> {
    Git(&'a [&'a str]),
    Saved,
    Stable,
}

#[cfg(not(verus_keep_ghost))]
pub struct RemoteSelection<'a> {
    pub remote: &'a str,
    pub endpoint: &'a str,
    pub branch_ref: &'a str,
    pub retained_ref: &'a str,
    pub retained: &'a str,
    pub head: &'a str,
    pub retry_absent: bool,
}

#[cfg(not(verus_keep_ghost))]
impl RemoteSelection<'_> {
    fn endpoint(
        &self,
        run: &mut impl FnMut(RemoteCheck) -> Result<Vec<u8>, String>,
    ) -> Result<(), String> {
        let bytes = run(RemoteCheck::Git(&[
            "remote",
            "get-url",
            "--push",
            "--all",
            self.remote,
        ]))?;
        let text = std::str::from_utf8(&bytes).map_err(|_| "Git output is not UTF-8")?;
        let mut lines = text.lines();
        if !single_endpoint(lines.next(), lines.next(), self.endpoint) {
            return Err("remote must have exactly the expected push URL".into());
        }
        Ok(())
    }

    fn observed(
        &self,
        run: &mut impl FnMut(RemoteCheck) -> Result<Vec<u8>, String>,
    ) -> Result<Option<String>, String> {
        let bytes = run(RemoteCheck::Git(&[
            "ls-remote",
            "--symref",
            "--",
            self.endpoint,
            self.branch_ref,
            self.retained_ref,
        ]))?;
        let text = std::str::from_utf8(&bytes).map_err(|_| "Git output is not UTF-8")?;
        read_advertisement(text, self.branch_ref, self.retained_ref, self.retained)
    }

    pub fn run(
        &self,
        report: &mut Value,
        mut run: impl FnMut(RemoteCheck) -> Result<Vec<u8>, String>,
    ) -> Result<(), String> {
        self.endpoint(&mut run)?;
        run(RemoteCheck::Saved)?;
        let observed = self.observed(&mut run)?;
        let absent = already_absent(observed.as_deref(), self.head, self.retry_absent)?;
        run(RemoteCheck::Stable)?;
        run(RemoteCheck::Saved)?;
        self.endpoint(&mut run)?;
        if !absent {
            report["remote_branch_removal"] = json!("attempted-unverified");
            // The work branch is leased at its observed OID. The target is rechecked after push.
            run(RemoteCheck::Git(&[
                "push",
                "--porcelain",
                "--no-follow-tags",
                "--recurse-submodules=no",
                &format!("--force-with-lease={}:{}", self.branch_ref, self.head),
                "--",
                self.endpoint,
                &format!(":{}", self.branch_ref),
            ]))?;
        }
        if self.observed(&mut run)?.is_some() {
            return Err("remote branch absence was not verified".into());
        }
        run(RemoteCheck::Saved)?;
        self.endpoint(&mut run)?;
        run(RemoteCheck::Stable)?;
        report["remote_branch_removed"] = json!(!absent);
        report["remote_branch_removal"] = json!(if absent {
            "already-absent"
        } else {
            "verified-absent"
        });
        Ok(())
    }
}
