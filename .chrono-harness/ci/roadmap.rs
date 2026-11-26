//! Roadmap verification rendered from the selected Copier template and host answers.
use vstd::prelude::*;

verus! {
/// Both deployed files are inputs to this proof compilation and its native test.
pub fn deployed_roadmap() -> (&'static str, &'static str) {
    (include_str!("../../.github/roadmap.cjs"), include_str!("../../.github/workflows/issue-roadmap.yml"))
}

pub fn deployed_roadmap_lock() -> &'static str {
    include_str!("../../.github/workflows/actions.lock")
}

pub fn roadmap_repository(checkout: bool, owner_id: u64, repo_id: u64) -> (accepted: bool)
    ensures accepted == (owner_id == 44036562 && ((checkout && repo_id == 197814629)
        || (!checkout && repo_id == 205262760))),
{
    owner_id == 44036562 && repo_id == (if checkout { 197814629 } else { 205262760 })
}
/// The native caller hashes the actual UTF-8 bytes embedded above.
pub fn roadmap_matches(script: &String, workflow: &String) -> (accepted: bool)
    ensures accepted == (script@ == "5824ea576bd25a792c76e976dc2dcec1c7eb684f6a3a9de9919220a546b61cf9"@ && workflow@ == "f213a45c0fef8b18e358ba3f92e3bbc1a06db084f39519dec94b9f60dd6c5d4f"@),
{
    *script == "5824ea576bd25a792c76e976dc2dcec1c7eb684f6a3a9de9919220a546b61cf9".to_string() && *workflow == "f213a45c0fef8b18e358ba3f92e3bbc1a06db084f39519dec94b9f60dd6c5d4f".to_string()
}
}

#[cfg(not(verus_keep_ghost))]
mod native {
    use super::*;
    use serde_yaml::Value;

    fn roadmap_digest(source: &str) -> String {
        use sha2::{Digest, Sha256};
        Sha256::digest(source.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    fn locked_roadmap_reference<'a>(
        lock: &Value,
        workflow: &'a [Value],
        repo: &str,
        checkout: bool,
    ) -> Option<&'a str> {
        let mut matched = None;
        for item in workflow {
            let name = item.as_str()?;
            let (source, reference) = name.split_once('@')?;
            if !source.eq_ignore_ascii_case(repo) {
                continue;
            }
            if source != repo || matched.is_some() {
                return None;
            }
            let entry = lock["dependencies"].get(name)?;
            let sha = entry["commit"].as_str()?.strip_prefix("sha1-")?;
            if reference.is_empty()
                || !reference
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-' | b'/'))
                || entry["ref"].as_str() != Some(reference)
                || entry
                    .get("hostname")
                    .is_some_and(|v| v.as_str() != Some("github.com"))
                || sha.len() != 40
                || !sha
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                || sha.bytes().all(|b| b == b'0')
                || !roadmap_repository(
                    checkout,
                    entry["owner_id"].as_u64()?,
                    entry["repo_id"].as_u64()?,
                )
            {
                return None;
            }
            matched = Some(name);
        }
        matched
    }

    fn roadmap_structure(workflow: &str, lock: &str) -> Option<String> {
        let lock: Value = serde_yaml::from_str(lock).ok()?;
        if lock["version"].as_str()? != "v0.0.3" {
            return None;
        }
        let mapping = lock["workflows"][".github/workflows/issue-roadmap.yml"].as_sequence()?;
        let checkout = locked_roadmap_reference(&lock, mapping, "actions/checkout", true)?;
        let script = locked_roadmap_reference(&lock, mapping, "actions/github-script", false)?;
        let mut projected = String::with_capacity(workflow.len());
        for line in workflow.split_inclusive('\n') {
            if let Some(value) = line.strip_prefix("      - uses: ") {
                let repository = match value.strip_suffix('\n')? {
                    v if v == checkout => "actions/checkout",
                    v if v == script => "actions/github-script",
                    _ => return None,
                };
                projected.push_str("      - uses: ");
                projected.push_str(repository);
                projected.push_str("@LOCKED\n");
            } else {
                projected.push_str(line);
            }
        }
        Some(projected)
    }

    #[test]
    fn locked_roadmap_rejects_changes_to_its_deployment_contract() {
        let (script, workflow) = deployed_roadmap();
        let lock = deployed_roadmap_lock();
        let script_hash = roadmap_digest(script);
        let accepted = |candidate: &str, lock: &str| {
            roadmap_structure(candidate, lock)
                .is_some_and(|structure| roadmap_matches(&script_hash, &roadmap_digest(&structure)))
        };
        for (expected, changed) in [
            ("actions/checkout@", "other/checkout@"),
            ("contents: read", "contents: write"),
            ("    steps:\n", "    steps:\n      - run: echo extra\n"),
            (".releaseIssue(", ".different("),
        ] {
            let edit = workflow.replacen(expected, changed, 1);
            assert_ne!(edit, workflow);
            assert!(!accepted(&edit, lock));
        }
        for expected in [
            ".github/workflows/issue-roadmap.yml",
            "197814629",
            "205262760",
        ] {
            let edit = lock.replace(expected, "absent");
            assert_ne!(edit, lock);
            assert!(!accepted(workflow, &edit));
        }
        assert!(!accepted(workflow, ""));
    }

    #[test]
    fn deployed_roadmap_matches_the_selected_template() {
        let (script, workflow) = deployed_roadmap();
        let script_hash = roadmap_digest(script);
        let structure = roadmap_structure(workflow, deployed_roadmap_lock())
            .expect("the roadmap action references must belong to their GitHub workflow lock");
        let workflow_hash = roadmap_digest(&structure);
        assert!(
            roadmap_matches(&script_hash, &workflow_hash),
            "roadmap.cjs or issue-roadmap.yml differs from the selected template"
        );
        assert!(!roadmap_matches(&workflow_hash, &workflow_hash));
        assert!(!roadmap_matches(&script_hash, &script_hash));
    }
}
