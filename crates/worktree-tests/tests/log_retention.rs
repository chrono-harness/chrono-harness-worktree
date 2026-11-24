use chrono_worktree::log_retention::expire;
use std::{collections::BTreeMap, path::Path};

#[test]
fn log_owner_keeps_absence_partial_results_and_original_failures_distinct() {
    let env = BTreeMap::from([("PATH".into(), "/bin".into())]);
    for (code, bytes, err, expected) in [
        (0, "null", "", Ok(None)),
        (
            0,
            r#"{"removed":3,"legacy":0,"busy":false,"errors":[]}"#,
            "",
            Ok(Some((3, 0, false, vec![]))),
        ),
        (
            1,
            r#"{"removed":1,"legacy":2,"busy":true,"errors":["denied","other"]}"#,
            "",
            Ok(Some((1, 2, true, vec!["denied", "other"]))),
        ),
        (1, "null", "", Err("contradicts")),
        (
            0,
            r#"{"removed":3,"legacy":0,"busy":false,"errors":["denied"]}"#,
            "",
            Err("contradicts"),
        ),
        (
            1,
            r#"{"removed":3,"legacy":0,"busy":false,"errors":[]}"#,
            "",
            Err("contradicts"),
        ),
        (1, "invalid json", "owner error", Err("owner error")),
        (2, "", "unreadable\n", Err("unreadable")),
        (0, "null", "unexpected", Err("unexpected")),
    ] {
        let result = expire(
            Path::new("/repo with spaces"),
            "/tools/chrono-diagnostics",
            7,
            8192,
            &env,
            |spec| {
                assert_eq!(spec.program, "/tools/chrono-diagnostics");
                assert_eq!(spec.args, ["expire-logs-v1", "/repo with spaces"]);
                assert_eq!(spec.env, env);
                assert_eq!((spec.timeout_seconds, spec.output_limit_bytes), (7, 8192));
                Ok((code, bytes.as_bytes().to_vec(), err.as_bytes().to_vec()))
            },
        );
        match expected {
            Ok(expected) => {
                let report = result.unwrap();
                let observed = report.as_ref().map(|r| {
                    (
                        r.removed,
                        r.legacy,
                        r.busy,
                        r.errors.iter().map(String::as_str).collect::<Vec<_>>(),
                    )
                });
                assert_eq!(observed, expected);
            }
            Err(message) => assert!(result.unwrap_err().contains(message)),
        }
    }
}
