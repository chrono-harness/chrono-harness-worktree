//! Current operator custody of exact historical main outputs, not past producer release.
#[cfg(not(verus_keep_ghost))]
use super::*;
#[cfg(not(verus_keep_ghost))]
use vstd::contrib::exec_spec::ExecSpecEq;
#[cfg(verus_keep_ghost)]
use vstd::contrib::exec_spec::ExecSpecEq;
#[cfg(not(verus_keep_ghost))]
use vstd::prelude::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub fn main_cache_target(cache_only: bool, status: &str, main: bool) -> (admitted: bool)
    ensures admitted == (cache_only && status@ == "active"@ && main),
{
    cache_only && <&str>::exec_eq(status, "active") && main
}

pub fn custody_declaration(schema: &str, same_path: bool, released: bool, reason_present: bool,
    artifacts: usize) -> (valid: bool)
    ensures valid == (schema@ == "chrono-main-cache-custody/v1"@ && same_path && released
        && reason_present && artifacts > 0),
{
    <&str>::exec_eq(schema, "chrono-main-cache-custody/v1") && same_path && released
        && reason_present && artifacts > 0
}

/// New custody is bound to today's checkout. Replay checks the recorded historical HEAD separately.
pub fn current_main_custody(schema: &str, same_path: bool, declared_head: &str, actual_head: &str,
    released: bool, reason_present: bool, artifacts: usize) -> (valid: bool)
    ensures valid == (schema@ == "chrono-main-cache-custody/v1"@ && same_path
        && declared_head@ == actual_head@ && released && reason_present && artifacts > 0),
{
    custody_declaration(schema, same_path, released, reason_present, artifacts)
        && <&str>::exec_eq(declared_head, actual_head)
}

pub fn custody_paths(requested: usize, distinct: usize, all_declared: bool) -> (valid: bool)
    ensures valid == (requested == distinct && all_declared),
{
    requested == distinct && all_declared
}

pub fn custody_generation(current: u64) -> (generation: Option<u64>)
    ensures generation == (if current < u64::MAX { Some((current + 1) as u64) } else { None }),
{
    current.checked_add(1)
}
}

#[cfg(not(verus_keep_ghost))]
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Custody {
    schema: String,
    path: PathBuf,
    head: String,
    artifacts: Vec<String>,
    current_consumers_released: bool,
    reason: String,
}

#[cfg(not(verus_keep_ghost))]
impl Manager {
    pub(super) fn cache_release_generation(&self, e: &Entry) -> Result<Option<u64>, String> {
        let Some(request) = e.enrollment.get("cache_release") else {
            return Ok(None);
        };
        let receipt: Receipt =
            serde_json::from_value(request.clone()).map_err(|e| e.to_string())?;
        let record = json(&self.receipt(&receipt)?)?;
        if record["schema"] != "chrono-main-cache-release/v1"
            || record["path"] != value!(e.path)
            || record["attachment"] != value!(e.attachment)
        {
            return Err("main cache release does not bind this enrollment".into());
        }
        Ok(Some(
            record["cache_generation"]
                .as_u64()
                .ok_or("main cache release generation missing")?,
        ))
    }

    pub(super) fn main_cache_owned(&self, e: &Entry) -> Result<BTreeSet<String>, String> {
        let initial: Vec<String> = serde_json::from_value(
            e.enrollment["main_cache_artifacts"].clone(),
        )
        .map_err(|_| "main cache artifact declarations missing or invalid; preserve outputs")?;
        let mut owned: BTreeSet<_> = initial.iter().cloned().collect();
        if initial.len() != owned.len() {
            return Err("ambiguous main cache ownership; preserve outputs".into());
        }
        for receipt in &e.main_cache_adoptions {
            let record = json(&self.receipt(receipt)?)?;
            let plan: Custody = serde_json::from_value(record["custody"].clone())
                .map_err(|e| format!("invalid current custody record: {e}"))?;
            let selected: BTreeSet<_> = plan.artifacts.iter().cloned().collect();
            if record["schema"] != "chrono-main-cache-adoption/v1"
                || record["binding"] != Self::birth_binding(e)
                || record["previous"] != value!(owned)
                || record["historical_producer_outcome"] != "unknown"
                || record["disposal"] != "not-performed"
                || record["terminal_handoff"] != "not-claimed"
                || !custody_declaration(
                    &plan.schema,
                    plan.path == e.path,
                    plan.current_consumers_released,
                    !plan.reason.trim().is_empty(),
                    plan.artifacts.len(),
                )
                || selected.len() != plan.artifacts.len()
                || selected.iter().any(|p| owned.contains(p))
            {
                return Err("main cache adoption chain does not bind current custody".into());
            }
            facts::full_oid(&plan.head)?;
            owned.extend(selected);
            if record["next"] != value!(owned) {
                return Err("main cache adoption has an invalid ownership transition".into());
            }
        }
        Ok(owned)
    }

    pub(super) fn protected_main_history(&self, e: &Entry) -> Result<Value, String> {
        let owned = self.main_cache_owned(e)?;
        Ok(value!(
            e.enrollment["protected_historical_artifacts"]
                .as_array()
                .ok_or("main historical ownership declarations missing")?
                .iter()
                .filter(|row| !row["path"].as_str().is_some_and(|p| owned.contains(p)))
                .collect::<Vec<_>>()
        ))
    }

    pub(super) fn adopt_main_cache(
        &mut self,
        r: &mut Runner,
        target: &Path,
        plan_path: &str,
        report: &mut Value,
    ) -> Result<(), String> {
        let i = self
            .current_entry(target)
            .ok_or("current custody requires main enrollment")?;
        let e = self.ledger.entries[i].clone();
        if !main_cache_target(
            e.cache_only,
            &e.status,
            target == self.inputs.policy.coordinator_root,
        ) {
            return Err(
                "current cache custody requires the active adopted Git main checkout".into(),
            );
        }
        let lease = self
            .lease(&e, true)?
            .ok_or("main has live registered consumers; preserve outputs")?;
        let head = r.oid(target, "HEAD")?;
        self.check_entry(r, &e, &head, None)?;
        let bytes = crate::recovery::state_bytes(target, plan_path)?;
        let plan: Custody = decode(&bytes)?;
        if !current_main_custody(
            &plan.schema,
            plan.path == target,
            &plan.head,
            &head,
            plan.current_consumers_released,
            !plan.reason.trim().is_empty(),
            plan.artifacts.len(),
        ) {
            return Err(
                "current custody must bind exact main HEAD and explicit consumer release".into(),
            );
        }
        let selected: BTreeSet<_> = plan.artifacts.iter().cloned().collect();
        let all_declared = selected.iter().all(|path| {
            self.inputs.policy.artifacts.iter().any(|artifact| {
                artifact.path == *path && select_artifact(&artifact.disposition, true, false)
            })
        });
        if !custody_paths(plan.artifacts.len(), selected.len(), all_declared) {
            return Err("current custody selects repeated or undeclared disposable outputs".into());
        }
        artifact_disposal::registered(&plan.artifacts, &[self.inputs.registrations.config()])?;
        artifact_disposal::paths(r, target, &head, &plan.artifacts)?;
        let previous = self.main_cache_owned(&e)?;
        let newly_owned: Vec<_> = selected.difference(&previous).cloned().collect();
        if newly_owned.is_empty() {
            report["cache_adoption"] = value!({"status":"already-managed", "path":target,
                "disposal":"not-performed", "terminal_handoff":"not-claimed"});
            return lease.stable();
        }
        // Preserve the exact current declaration, and record only its new ownership delta.
        // The operator's declaration covers prior unmanaged consumers; kernel exclusion
        // covers adopted concurrent users. Neither proves any historical producer outcome.
        let mut custody = value!(plan);
        custody["artifacts"] = value!(newly_owned);
        let mut next = previous.clone();
        next.extend(newly_owned);
        let record = value!({"schema":"chrono-main-cache-adoption/v1",
            "binding":Self::birth_binding(&e), "previous":previous, "next":next,
            "custody":custody, "declaration_input":self.retain_bytes(&bytes)?,
            "coordinator_commit":self.inputs.anchor_head, "policy_sha256":sha256(&self.inputs.policy_bytes),
            "registry_digest":self.inputs.registry_digest, "historical_producer_outcome":"unknown",
            "disposal":"not-performed", "terminal_handoff":"not-claimed"});
        let name = format!(
            "main-cache-adoption-{}.json",
            crate::document::digest(&record)?
        );
        let receipt = self.immutable_reconciled(&name, &record)?;
        self.check_entry(r, &e, &head, None)?;
        artifact_disposal::paths(r, target, &head, &plan.artifacts)?;
        if crate::recovery::state_bytes(target, plan_path)? != bytes {
            return Err("current custody declaration changed before adoption".into());
        }
        lease.stable()?;
        self.reconcile_uses(i, &lease)?;
        let mut updated = self.ledger.entries[i].clone();
        updated.main_cache_adoptions.push(receipt.clone());
        let owner = updated
            .ownership
            .as_mut()
            .ok_or("main custody lost kernel ownership")?;
        owner.generation =
            custody_generation(owner.generation).ok_or("ownership generation overflow")?;
        owner.cache_pending = true;
        self.check_entry(r, &updated, &head, None)?;
        lease.stable()?;
        self.ledger.entries[i] = updated;
        self.save()?;
        report["cache_adoption"] = value!({"status":"adopted", "path":target,
            "receipt":receipt, "selected":custody["artifacts"],
            "historical_producer_outcome":"unknown", "disposal":"not-performed",
            "terminal_handoff":"not-claimed"});
        Ok(())
    }
}
