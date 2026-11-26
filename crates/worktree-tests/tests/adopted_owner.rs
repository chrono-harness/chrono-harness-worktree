use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::MetadataExt, path::Path};
use tempfile::tempdir;

const SELECTED: &str = ".chrono-harness/bin/chrono-worktree";
const STAGED: &str = ".chrono-harness/bin/owner-stages/test/chrono-worktree";
const SLOT: &str = ".chrono-harness/state/bootstrap-owner-current.json";
const RECEIPT: &str = ".chrono-harness/state/automatic-cleanup-v2/owner-adoption.json";

fn digest(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn binding(path: &Path) -> Value {
    let metadata = fs::metadata(path).unwrap();
    json!({"identity":format!("{}:{}",metadata.dev(),metadata.ino()),
           "sha256":digest(&fs::read(path).unwrap()),"size":metadata.len()})
}

fn record(root: &Path, adoption: &Value) {
    let bytes = serde_json::to_vec(adoption).unwrap();
    fs::write(root.join(RECEIPT), &bytes).unwrap();
    let mut slot = adoption.clone();
    slot["receipt"] = json!({"path":RECEIPT,"sha256":digest(&bytes)});
    fs::write(root.join(SLOT), serde_json::to_vec(&slot).unwrap()).unwrap();
}

#[test]
fn reads_only_a_complete_unchanged_publication() {
    let temp = tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let query = |args: &[&str]| {
        let mut cmd = vec![
            "adopted-owner-v1".to_string(),
            root.to_str().unwrap().to_string(),
            SELECTED.to_string(),
        ];
        cmd.extend(args.iter().map(|v| v.to_string()));
        chrono_worktree::run(&cmd)
    };
    let absent = query(&[]);
    assert_eq!(absent.exit_code, 0, "{}", absent.stderr);
    assert_eq!(absent.stdout.trim(), "null");
    assert!(!root.join(".chrono-harness").exists());

    let retained = root.join(STAGED).parent().unwrap().to_owned();
    fs::create_dir_all(&retained).unwrap();
    fs::create_dir_all(root.join(RECEIPT).parent().unwrap()).unwrap();
    fs::write(root.join(SELECTED), b"candidate").unwrap();
    fs::write(root.join(STAGED), b"candidate").unwrap();
    fs::write(retained.join("previous-owner"), b"previous").unwrap();
    let previous = binding(&retained.join("previous-owner"));
    let installed = binding(&root.join(SELECTED));
    let mut staged = binding(&root.join(STAGED));
    staged["path"] = json!(STAGED);
    let stage = json!({"previous":previous,"staged":staged,"selected":SELECTED});
    let intent = json!({"schema":"chrono-bootstrap-owner-publication-intent/v1","stage":stage,"replacement":installed});
    fs::write(
        retained.join("publication-intent.json"),
        serde_json::to_vec(&intent).unwrap(),
    )
    .unwrap();
    let publication = json!({"schema":"chrono-bootstrap-owner-publication/v1","stage":stage,"installed":installed,"adoption":"not-claimed","task_completion":"not-claimed"});
    let result_path = retained.join("publication-result.json");
    fs::write(&result_path, serde_json::to_vec(&publication).unwrap()).unwrap();
    let mut adoption = json!({"schema":"chrono-worktree-owner-adoption/v1","source_commit":"c","stage_result":".chrono-harness/state/bootstrap-stage/report.json","stage_sha256":"abcd","installed":installed,"original_owner":previous,"original_protection":{},"publication":publication,"terminal_handoff":"not-claimed","task_completion":"not-claimed"});
    record(&root, &adoption);
    let found = query(&["--if-installed", "c"]);
    assert_eq!(found.exit_code, 0, "{}", found.stderr);
    let answer: Value = serde_json::from_str(&found.stdout).unwrap();
    assert_eq!(answer["stage"], stage);
    assert_eq!(answer["installed"], installed);
    assert_eq!(answer["report_slot"], SLOT);
    assert_eq!(query(&["--if-installed", "other"]).stdout.trim(), "null");
    fs::remove_file(&result_path).unwrap();
    assert_eq!(query(&[]).exit_code, 2);
    assert!(!result_path.exists());
    fs::write(&result_path, serde_json::to_vec(&publication).unwrap()).unwrap();
    adoption["unexpected"] = json!(true);
    record(&root, &adoption);
    assert_eq!(query(&[]).exit_code, 2);
    adoption.as_object_mut().unwrap().remove("unexpected");
    record(&root, &adoption);
    fs::write(root.join(RECEIPT), b"{}").unwrap();
    assert_eq!(query(&[]).exit_code, 2);
    record(&root, &adoption);
    fs::write(retained.join("previous-owner"), b"changed").unwrap();
    assert_eq!(query(&[]).exit_code, 2);
    fs::write(root.join(SELECTED), b"replaced").unwrap();
    assert_eq!(query(&["--if-installed"]).stdout.trim(), "null");
    assert_eq!(query(&[]).exit_code, 2);
}
