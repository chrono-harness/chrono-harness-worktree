#[cfg(not(verus_keep_ghost))]
use crate::process::CommandSpec;
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::collections::BTreeMap;

use vstd::prelude::*;

verus! {
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

pub(crate) fn age_result<F>(reply: ObservationAgeReply, message: &str, invalid: F)
    -> (answer: Result<bool, String>)
    where F: FnOnce() -> String,
    requires invalid.requires(()),
    ensures match reply {
        ObservationAgeReply::Fresh => answer == Ok::<bool, String>(true),
        ObservationAgeReply::Stale => answer == Ok::<bool, String>(false),
        ObservationAgeReply::Diagnostic => answer is Err && answer->Err_0@ == message@,
        ObservationAgeReply::Invalid => answer is Err && invalid.ensures((), answer->Err_0),
    },
{
    match reply {
        ObservationAgeReply::Fresh => Ok(true),
        ObservationAgeReply::Stale => Ok(false),
        ObservationAgeReply::Diagnostic => Err(message.to_owned()),
        ObservationAgeReply::Invalid => Err(invalid()),
    }
}
}

/// Query the configured workflow command and distinguish stale observations from failed checks.
#[cfg(not(verus_keep_ghost))]
pub fn check(
    context: &Value,
    workflow: &Value,
    program: &str,
    timeout_seconds: u64,
    output_limit_bytes: usize,
    environment: &BTreeMap<String, String>,
    run: impl FnOnce(&CommandSpec, &[u8]) -> Result<(i32, Vec<u8>, Vec<u8>), String>,
) -> Result<bool, String> {
    let input = json!({
        "context": {
            "observed_at": &context["observed_at"],
            "branch_started_at": &context["branch_started_at"],
        },
        "workflow": {"staleness": {"max_age_hours": &workflow["staleness"]["max_age_hours"]}},
    });
    let command = CommandSpec {
        program: program.into(),
        args: vec!["check-observation-age".into()],
        env: environment.clone(),
        timeout_seconds,
        output_limit_bytes,
    };
    let (code, stdout, stderr) = run(
        &command,
        &serde_json::to_vec(&input).map_err(|e| e.to_string())?,
    )?;
    let diagnostic = String::from_utf8(stderr).map_err(|e| e.to_string())?;
    let message = diagnostic.strip_suffix('\n').unwrap_or(&diagnostic);
    let reply = observation_age_reply(
        code,
        stdout.is_empty(),
        message.is_empty(),
        message.starts_with("E_BRANCH_STALE:"),
    );
    age_result(reply, message, || {
        format!("workflow age check returned {code}: {message}; stdout={stdout:?}")
    })
}
