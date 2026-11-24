use chrono_worktree::facts::{
    acquire_immutable_input, acquire_registry_blobs, checkout_changes, parse_tree,
};
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
};
const COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const EMPTY: &str = "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391";
const CONTENT: &str = "ce013625030ba8dba906f756967f9e9ca394464a";

#[test]
fn immutable_reads_verify_exact_git_identity_and_keep_data_limit_separate() {
    let mut calls = 0;
    let run = acquire_immutable_input(COMMIT, "a file", 6, |argv, stdin, limit| {
        calls += 1;
        if calls == 1 {
            assert_eq!(argv, ["cat-file", "--batch-check", "-z"]);
            assert_eq!(stdin, format!("{COMMIT}:a file\0").as_bytes());
            assert_eq!(limit, None);
            Ok(format!("{CONTENT} blob 6\n").into_bytes())
        } else {
            assert_eq!(argv, ["cat-file", "blob", CONTENT]);
            assert!(stdin.is_empty());
            assert_eq!(limit, Some(6));
            Ok(b"hello\n".to_vec())
        }
    });
    assert_eq!(calls, 2);
    assert_eq!(run.unwrap(), b"hello\n");
    for (metadata, data, limit, expected_calls) in [
        (format!("{CONTENT} blob 6\n"), b"HELLO\n".as_slice(), 6, 2),
        (format!("{CONTENT} blob 6\n"), b"".as_slice(), 5, 1),
        (format!("{CONTENT} blob 06\n"), b"".as_slice(), 6, 1),
        (format!("{CONTENT} tree 6\n"), b"".as_slice(), 6, 1),
    ] {
        let mut calls = 0;
        assert!(
            acquire_immutable_input(COMMIT, "a file", limit, |_, _, _| {
                calls += 1;
                Ok(if calls == 1 {
                    metadata.as_bytes()
                } else {
                    data
                }
                .to_vec())
            })
            .is_err()
        );
        assert_eq!(calls, expected_calls);
    }
}

#[test]
fn registration_batches_reject_wrong_count_body_and_framing() {
    let paths = ["one", "two", "three"].map(String::from);
    let header = format!("{EMPTY} blob 0\n");
    let body = format!("{header}\n").repeat(3);
    for (metadata, response, ok) in [
        (header.repeat(3), body.clone(), true),
        (header.repeat(2), body.clone(), false),
        (header.repeat(3), format!("{body}x"), false),
        (header.repeat(3), body.trim_end().into(), false),
        (
            header.replace(EMPTY, CONTENT).repeat(3),
            body.clone(),
            false,
        ),
    ] {
        let mut calls = 0;
        let result = acquire_registry_blobs(COMMIT, &paths, 1024, |argv, input| {
            calls += 1;
            if calls == 1 {
                assert_eq!(argv, ["cat-file", "--batch-check"]);
                assert_eq!(
                    input,
                    paths
                        .iter()
                        .map(|p| format!("{COMMIT}:{p}\n"))
                        .collect::<String>()
                        .as_bytes()
                );
                Ok(metadata.as_bytes().to_vec())
            } else {
                assert_eq!(argv, ["cat-file", "--batch"]);
                Ok(response.as_bytes().to_vec())
            }
        });
        assert_eq!(result.is_ok(), ok);
        if ok {
            assert_eq!(
                result.unwrap(),
                paths
                    .iter()
                    .cloned()
                    .map(|p| (p, vec![]))
                    .collect::<Vec<_>>()
            );
        }
    }
}

#[test]
fn checkout_protects_unmerged_staged_and_changed_physical_objects() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("file with spaces");
    fs::write(&path, b"hello\n").unwrap();
    let tree = parse_tree(format!("100644 blob {CONTENT}\tfile with spaces\0").as_bytes()).unwrap();
    let original = format!("100644 {CONTENT} 0\tfile with spaces\0");
    let changes = |index: &str| checkout_changes(root.path(), &tree, index.as_bytes());
    assert!(changes(&original).unwrap().is_empty());
    for changed in [
        "".into(),
        original.replace(CONTENT, EMPTY),
        original.replace(" 0\t", " 1\t"),
    ] {
        assert_eq!(changes(&changed).unwrap(), ["file with spaces"]);
    }
    for invalid in [
        original.trim_end_matches('\0').to_owned(),
        original.repeat(2),
        original.replace(" 0\t", " 9\t"),
    ] {
        assert!(changes(&invalid).is_err());
    }
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(changes(&original).unwrap(), ["file with spaces"]);
    fs::remove_file(&path).unwrap();
    symlink("outside", &path).unwrap();
    assert_eq!(changes(&original).unwrap(), ["file with spaces"]);
    let target_hash = format!("{:x}", Sha256::digest(b"blob 7\0outside"));
    let links =
        parse_tree(format!("120000 blob {target_hash}\tfile with spaces\0").as_bytes()).unwrap();
    let index = format!("120000 {target_hash} 0\tfile with spaces\0");
    assert!(
        checkout_changes(root.path(), &links, index.as_bytes())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn separate_callers_share_reads_and_exclude_an_overlapping_write() {
    use chrono_worktree::conflicting_paths;
    use std::path::PathBuf;
    let names = |s: &[&str]| s.iter().map(PathBuf::from).collect::<Vec<_>>();
    for (left_r, left_w, right_r, right_w, expected) in [
        (
            vec!["/task/src"],
            vec![],
            vec!["/task/src/main"],
            vec![],
            false,
        ),
        (
            vec!["/task/src"],
            vec![],
            vec![],
            vec!["/task/src/main"],
            true,
        ),
        (
            vec![],
            vec!["/task/src"],
            vec![],
            vec!["/task/src/main"],
            true,
        ),
        (vec![], vec!["/task/src"], vec![], vec!["/task/src2"], false),
        (vec![], vec!["/a/x"], vec![], vec!["/b/x"], false),
        (
            vec![],
            vec!["/git/refs/heads/dev"],
            vec!["/git/refs/heads/dev"],
            vec![],
            true,
        ),
        (
            vec!["/git/index"],
            vec!["/a/x"],
            vec![],
            vec!["/git/index"],
            true,
        ),
    ] {
        let (lr, lw, rr, rw) = (
            names(&left_r),
            names(&left_w),
            names(&right_r),
            names(&right_w),
        );
        assert_eq!(conflicting_paths(&lr, &lw, &rr, &rw), expected);
        assert_eq!(conflicting_paths(&rr, &rw, &lr, &lw), expected);
    }
}

#[test]
fn observed_git_replies_control_dispatch_and_the_final_absence_receipt() {
    use chrono_worktree::verified_lifecycle::{RemoteCheck, RemoteSelection};
    use serde_json::json;
    const URL: &str = "https://example.invalid/team/repo.git";
    const BRANCH: &str = "refs/heads/integration/fixed";
    const TARGET: &str = "refs/heads/dev";
    let head = "a".repeat(40);
    let retained = "b".repeat(40);
    let target = format!("{retained}\t{TARGET}\n");
    let branch = format!("{head}\t{BRANCH}\n");
    let both = format!("{branch}{target}");
    let unique = format!("{URL}\n");
    let lease = format!("--force-with-lease={BRANCH}:{head}");
    let refspec = format!(":{BRANCH}");
    let test = |endpoints: &str, before: &str, after: &str, retry_absent: bool| {
        let selection = RemoteSelection {
            remote: "custom",
            endpoint: URL,
            branch_ref: BRANCH,
            retained_ref: TARGET,
            retained: &retained,
            head: &head,
            retry_absent,
        };
        let mut report =
            json!({"remote_branch_removed":false,"remote_branch_removal":"not-attempted"});
        let mut observations = 0;
        let mut pushes = 0;
        let result = selection.run(&mut report, |action| match action {
            RemoteCheck::Git(argv) if argv.first() == Some(&"remote") => {
                assert_eq!(argv, ["remote", "get-url", "--push", "--all", "custom"]);
                Ok(endpoints.as_bytes().to_vec())
            }
            RemoteCheck::Git(argv) if argv.first() == Some(&"ls-remote") => {
                assert_eq!(argv, ["ls-remote", "--symref", "--", URL, BRANCH, TARGET]);
                observations += 1;
                Ok(if observations == 1 { before } else { after }
                    .as_bytes()
                    .to_vec())
            }
            RemoteCheck::Git(argv) if argv.first() == Some(&"push") => {
                pushes += 1;
                assert_eq!(
                    argv,
                    [
                        "push",
                        "--porcelain",
                        "--no-follow-tags",
                        "--recurse-submodules=no",
                        &lease,
                        "--",
                        URL,
                        &refspec
                    ]
                );
                Ok(vec![0xff])
            }
            RemoteCheck::Git(other) => panic!("unexpected Git arguments {other:?}"),
            RemoteCheck::Saved | RemoteCheck::Stable => Ok(Vec::new()),
        });
        (result, pushes, observations, report)
    };
    for (urls, refs, expected_error) in [
        (
            format!("{URL}\n{URL}\n"),
            both.clone(),
            "remote must have exactly the expected push URL",
        ),
        (
            unique.clone(),
            format!("{both}{branch}"),
            "unexpected or duplicate remote ref observation",
        ),
        (
            unique.clone(),
            format!("{target}{head}\trefs/heads/other\n"),
            "unexpected or duplicate remote ref observation",
        ),
        (
            unique.clone(),
            format!("{branch}{head}\t{TARGET}\n"),
            "remote target does not match retained commit",
        ),
        (
            unique.clone(),
            format!("{retained}\t{BRANCH}\n{target}"),
            "remote branch changed or absent without an explicit retry plan",
        ),
        (
            unique.clone(),
            format!("{target}missing-tab\n"),
            "invalid remote ref observation",
        ),
        (
            unique.clone(),
            format!("{target}bad\t{BRANCH}\n"),
            "expected full nonzero lowercase commit OID",
        ),
    ] {
        let (result, pushes, _, report) = test(&urls, &refs, &target, false);
        assert_eq!(result.unwrap_err(), expected_error);
        assert_eq!(pushes, 0);
        assert_eq!(report["remote_branch_removal"], "not-attempted");
    }
    let (result, pushes, observations, report) = test(&unique, &both, &target, false);
    result.unwrap();
    assert_eq!((pushes, observations), (1, 2));
    assert_eq!(report["remote_branch_removed"], true);
    assert_eq!(report["remote_branch_removal"], "verified-absent");
    let (result, pushes, _, report) = test(&unique, &both, &both, false);
    assert_eq!(
        result.unwrap_err(),
        "remote branch absence was not verified"
    );
    assert_eq!(pushes, 1);
    assert_eq!(report["remote_branch_removal"], "attempted-unverified");
    let (result, pushes, _, report) = test(&unique, &target, &target, true);
    result.unwrap();
    assert_eq!(pushes, 0);
    assert_eq!(report["remote_branch_removal"], "already-absent");
}
