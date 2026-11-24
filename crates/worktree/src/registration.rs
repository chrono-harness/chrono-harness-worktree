//! Worktree passes fixed host data to the installed registration command.
use crate::document::{decode, file_identity};
use chrono_harness::{CommandSpec, resolve_program, run_process_observed_data};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Clone)]
pub(crate) struct Client {
    root: PathBuf,
    command: CommandSpec,
    digest: String,
}

impl Client {
    pub(crate) fn bind(root: &Path, config: &crate::Config) -> Result<Self, String> {
        let executable = resolve_program(
            root,
            config
                .registration_program
                .as_deref()
                .unwrap_or("chrono-judge-registration"),
            None,
        )?;
        Ok(Self {
            root: root.to_owned(),
            command: CommandSpec {
                program: executable
                    .to_str()
                    .ok_or("registration path is not UTF-8")?
                    .into(),
                args: vec!["worktree-inputs-v1".into()],
                env: BTreeMap::new(),
                timeout_seconds: config.timeout_seconds,
                output_limit_bytes: config.output_limit_bytes,
            },
            digest: file_identity(&executable)?.0,
        })
    }

    pub(crate) fn query<T: serde::de::DeserializeOwned>(
        &self,
        request: &Value,
    ) -> Result<T, String> {
        let bytes = serde_json::to_vec(request).map_err(|e| e.to_string())?;
        // Selected documents and path lists are subsets of this input. Diagnostic
        // stderr keeps the host's process limit independently of the data reply.
        let bound = bytes
            .len()
            .checked_add(256)
            .ok_or("registration response bound overflow")?;
        let _custody = crate::retention::forward_scope()?;
        let r = run_process_observed_data(&self.root, &self.command, &bytes, &self.digest, bound)?;
        if let Some(failure) = r.failure {
            return Err(failure);
        }
        if r.exit_code != 0 {
            return Err(format!(
                "worktree-inputs-v1 exited {}: {}",
                r.exit_code,
                r.stderr.trim_end()
            ));
        }
        decode(&r.stdout_bytes)
    }

    pub(crate) fn nonartifact_paths(
        &self,
        config: &Value,
        paths: &[String],
    ) -> Result<Vec<String>, String> {
        #[derive(Deserialize)]
        struct Paths {
            paths: Vec<String>,
        }
        let result:Paths=self.query(&json!({"operation":"nonartifact-paths","config":{"artifacts":config["artifacts"]},"paths":paths}))?;
        // A response to this batch can only retain actual input paths in order.
        let mut inputs = paths.iter();
        if !result.paths.iter().all(|p| inputs.any(|i| i == p)) {
            return Err("registration returned paths outside the queried batch".into());
        }
        Ok(result.paths)
    }
}

pub(crate) struct Registrations {
    pub(crate) client: Arc<Client>,
    entry: String,
    config: Value,
    filemap: Value,
    projects: Value,
    workflow: Value,
}

impl Registrations {
    pub(crate) fn load(
        client: Arc<Client>,
        mut documents: BTreeMap<String, Value>,
        entry: &str,
    ) -> Result<Self, String> {
        #[derive(Deserialize)]
        struct Snapshot {
            effective_config: String,
            filemap: Value,
        }
        let view: Snapshot =
            client.query(&json!({"operation":"snapshot","entry":entry,"documents":documents}))?;
        let config = documents
            .remove(&view.effective_config)
            .ok_or("missing validated lifecycle configuration")?;
        let mut document = |field: &str| {
            let path = config["registries"][field]
                .as_str()
                .ok_or("missing validated registry path")?;
            documents
                .remove(path)
                .ok_or_else(|| format!("missing validated {field} registry"))
        };
        let projects = document("projects")?;
        let workflow = document("workflow")?;
        Ok(Self {
            client,
            entry: entry.into(),
            config,
            filemap: view.filemap,
            projects,
            workflow,
        })
    }
    pub(crate) fn entry_path(&self) -> &str {
        &self.entry
    }
    pub(crate) fn config(&self) -> &Value {
        &self.config
    }
    pub(crate) fn filemap(&self) -> &Value {
        &self.filemap
    }
    pub(crate) fn projects(&self) -> &Value {
        &self.projects
    }
    pub(crate) fn workflow(&self) -> &Value {
        &self.workflow
    }
}
