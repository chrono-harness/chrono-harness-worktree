#[path = "../../worktree/src/check_inputs/observation_age.rs"]
mod observation_age;
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[test]
fn calls_the_selected_workflow_and_distinguishes_staleness_from_failure() {
    let environment = BTreeMap::from([("PATH".into(), "/opt/tools".into())]);
    let context = json!({"branch_started_at":"start", "observed_at":"end", "extra":42});
    let workflow = json!({"staleness":{"max_age_hours":1.5}, "extra":42});
    for (code, out, err, expected) in [
        (0, "", "", Ok(true)),
        (1, "", "E_BRANCH_STALE: age_ns=123\n", Ok(false)),
        (
            2,
            "",
            "E_BRANCH_CONTEXT: original error\n",
            Err("E_BRANCH_CONTEXT: original error".to_string()),
        ),
        (
            1,
            "",
            "unrelated",
            Err("workflow age check returned 1: unrelated; stdout=[]".to_string()),
        ),
        (
            0,
            "x",
            "",
            Err("workflow age check returned 0: ; stdout=[120]".to_string()),
        ),
    ] {
        let result = observation_age::check(
            &context,
            &workflow,
            "/opt/workflow",
            9,
            2048,
            &environment,
            |spec, input| {
                assert_eq!(spec.program, "/opt/workflow");
                assert_eq!(spec.args, ["check-observation-age"]);
                assert_eq!(spec.env, environment);
                assert_eq!((spec.timeout_seconds, spec.output_limit_bytes), (9, 2048));
                assert_eq!(
                    serde_json::from_slice::<Value>(input).unwrap(),
                    json!({
                        "context":{"branch_started_at":"start","observed_at":"end"},
                        "workflow":{"staleness":{"max_age_hours":1.5}},
                    })
                );
                Ok((code, out.as_bytes().to_vec(), err.as_bytes().to_vec()))
            },
        );
        assert_eq!(result, expected);
    }
    let result = observation_age::check(
        &context,
        &workflow,
        "workflow",
        9,
        2048,
        &environment,
        |_, _| Err("timeout".into()),
    );
    assert_eq!(result, Err("timeout".into()));
}
