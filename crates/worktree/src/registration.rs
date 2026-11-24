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
pub(crate) enum Client {
    Standalone,
    Registered(Registered),
}

#[derive(Clone)]
pub(crate) struct Registered {
    root: PathBuf,
    command: CommandSpec,
    digest: String,
}

impl Client {
    pub(crate) fn bind(root: &Path, config: &crate::Config) -> Result<Self, String> {
        if config.schema == "chrono-worktree-config/v3" && config.registration_program.is_none() {
            return Ok(Self::Standalone);
        }
        let executable = resolve_program(
            root,
            config
                .registration_program
                .as_deref()
                .unwrap_or("chrono-judge-registration"),
            None,
        )?;
        Ok(Self::Registered(Registered {
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
        }))
    }

    fn standalone_artifacts(config: &Value) -> Result<Vec<&str>, String> {
        let rows = config["artifacts"]
            .as_array()
            .ok_or("missing worktree artifact registrations")?;
        let mut result = Vec::new();
        for row in rows {
            let prefix = row["path"].as_str().ok_or("missing artifact path")?;
            let physical = prefix
                .strip_suffix('/')
                .ok_or("worktree artifact must name a directory")?;
            crate::document::relative_path(physical)?;
            if row["tracked"] != false
                || physical
                    .split('/')
                    .any(|part| matches!(part, "" | "." | ".." | ".git"))
            {
                return Err(format!("invalid worktree artifact directory: {prefix}"));
            }
            if result.contains(&prefix) {
                return Err(format!("duplicate worktree artifact directory: {prefix}"));
            }
            result.push(prefix);
        }
        Ok(result)
    }

    pub(crate) fn custody(&self, config: &Value) -> Result<String, String> {
        let expected = config["registries"]["filemap"]
            .as_str()
            .ok_or("worktree custody FILEMAP path missing")?;
        crate::document::relative_path(expected)?;
        if let Self::Registered(registered) = self {
            let result: Value =
                registered.query(&json!({"operation":"custody-config","config":config}))?;
            if result["filemap_path"] != expected {
                return Err("registration custody FILEMAP path differs".into());
            }
        } else {
            Self::standalone_artifacts(config)?;
        }
        Ok(expected.to_owned())
    }

    pub(crate) fn custody_filemap(&self, filemap: &Value) -> Result<(), String> {
        if let Self::Registered(registered) = self {
            let result: Value =
                registered.query(&json!({"operation":"custody-filemap","filemap":filemap}))?;
            if result["valid"] != true {
                return Err("registration custody FILEMAP is not valid".into());
            }
        } else {
            Self::file_paths(filemap)?;
        }
        Ok(())
    }

    fn file_paths(filemap: &Value) -> Result<(), String> {
        for row in filemap["files"]
            .as_array()
            .ok_or("missing worktree file registrations")?
        {
            crate::document::relative_path(
                row["path"].as_str().ok_or("worktree registration path")?,
            )?;
        }
        Ok(())
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
        let Self::Registered(registered) = self else {
            let artifacts = Self::standalone_artifacts(config)?;
            return Ok(paths
                .iter()
                .filter(|path| !artifacts.iter().any(|prefix| path.starts_with(prefix)))
                .cloned()
                .collect());
        };
        let result:Paths=registered.query(&json!({"operation":"nonartifact-paths","config":{"artifacts":config["artifacts"]},"paths":paths}))?;
        // A response to this batch can only retain actual input paths in order.
        let mut inputs = paths.iter();
        if !result.paths.iter().all(|p| inputs.any(|i| i == p)) {
            return Err("registration returned paths outside the queried batch".into());
        }
        Ok(result.paths)
    }
}

impl Registered {
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
        selected: &str,
    ) -> Result<Self, String> {
        #[derive(Deserialize)]
        struct Snapshot {
            effective_config: String,
            filemap: Value,
        }
        let view = match client.as_ref() {
            Client::Registered(registered) => registered
                .query(&json!({"operation":"snapshot","entry":entry,"documents":documents}))?,
            Client::Standalone => {
                let config = documents
                    .get(selected)
                    .ok_or("missing worktree host configuration")?;
                let files = config["registries"]["filemap"]
                    .as_str()
                    .ok_or("missing worktree filemap")?;
                let filemap = documents
                    .get(files)
                    .ok_or("missing worktree filemap document")?;
                Client::file_paths(filemap)?;
                Client::standalone_artifacts(config)?;
                // This tool consumes file ownership; execution edges belong to the optional judge.
                // Preserve the exact immutable filemap as the worktree's binding.
                Snapshot {
                    effective_config: selected.into(),
                    filemap: filemap.clone(),
                }
            }
        };
        if selected != view.effective_config {
            return Err("registration and worktree selected different fixed policies".into());
        }
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
