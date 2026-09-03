use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result};
use dashmap::DashMap;
use git2::{FetchOptions, Repository, build::RepoBuilder};
use nkd_http_server::{axum::extract::FromRef, tracing::instrument};

#[derive(Clone)]
pub struct AppState {
    project_registry: ProjectRegistry,
}
impl AppState {
    pub fn new() -> Self {
        Self {
            project_registry: ProjectRegistry::new(),
        }
    }
}

pub type Project = Arc<Mutex<ProjectRepo>>;

#[derive(Clone)]
pub struct ProjectRegistry {
    inner: DashMap<String, Project>,
}
impl ProjectRegistry {
    pub fn new() -> Self {
        Self {
            inner: DashMap::new(),
        }
    }

    pub fn get(&self, project_name: &str) -> Option<Project> {
        let project = self.inner.get(project_name)?;

        Some(Arc::clone(&project))
    }

    pub fn put(&self, repo: ProjectRepo) {
        self.inner
            .insert(repo.name.clone(), Arc::new(Mutex::new(repo)));
    }
}

impl FromRef<AppState> for ProjectRegistry {
    fn from_ref(input: &AppState) -> Self {
        input.project_registry.clone()
    }
}

pub struct ProjectRepo {
    pub name: String,
    pub remote_url: String,
    pub local_path: PathBuf,
    pub repo: Repository,
}

impl ProjectRepo {
    #[instrument]
    pub fn init(name: &str, remote_url: &str, base_dir: &Path) -> Result<Self> {
        let local_path = base_dir.join(name);

        let repo = if local_path.exists() {
            Repository::open(&local_path)
                .with_context(|| format!("failed to open repository at {}", local_path.display()))?
        } else {
            RepoBuilder::new()
                .clone(remote_url, &local_path)
                .with_context(|| {
                    format!(
                        "failed to clone {} into {}",
                        remote_url,
                        local_path.display()
                    )
                })?
        };

        Ok(Self {
            name: name.to_string(),
            remote_url: remote_url.to_string(),
            local_path,
            repo,
        })
    }

    #[instrument(skip(self))]
    pub fn fetch(&self) -> Result<()> {
        let mut remote = self.repo.find_remote("origin")?;
        let mut fetch_opts = FetchOptions::new();
        fetch_opts.download_tags(git2::AutotagOption::None);
        remote.fetch(
            &["refs/heads/*:refs/remotes/origin/*"],
            Some(&mut fetch_opts),
            None,
        )?;

        Ok(())
    }
}
