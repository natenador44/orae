use anyhow::anyhow;
use orae_http_server::{
    RouteHandler, get,
    responses::{HandlerResult, OptionExt},
    router_group,
};

use crate::{service, state::ProjectRegistry};

#[router_group(
    context = "/config",
    routes = [
        GetConfig,
    ]
)]
pub struct ConfigRouterGroup;

#[get("/{project}/{branch}/{file_path}")]
pub struct GetConfig {
    #[path(name = "project")]
    project_name: String,
    #[path]
    branch: String,
    #[path]
    file_path: String,
    #[state]
    project_registry: ProjectRegistry,
}

impl RouteHandler for GetConfig {
    type Ok = String;

    async fn handle(self) -> HandlerResult<Self::Ok> {
        let repo = self
            .project_registry
            .get(&self.project_name)
            .or_404_msg("project not found")?;

        let result = tokio::task::spawn_blocking(move || {
            let repo_guard = repo
                .lock()
                .map_err(|_| anyhow!("poisoned mutex in config route handler!"))?;
            service::git::read_file_from_branch(&repo_guard.repo, &self.branch, &self.file_path)
        })
        .await??;

        Ok(result)
    }
}
