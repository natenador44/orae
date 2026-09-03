use std::path::PathBuf;

use cargo::{
    GlobalContext,
    ops::{NewOptions, NewProjectKind},
};

pub struct CreateOptions {
    /// The directory to run `cargo new` in. Defaults to current directory.
    pub project_parent_dir: Option<PathBuf>,
    /// The type of project (workspace vs single project)
    pub project_structure: ProjectStructure,
}

pub enum ProjectStructure {
    Single(ProjectDefinition),
    Workspace(WorkspaceDefinition),
}

pub struct WorkspaceDefinition {
    project_name: String,
    projects: Vec<ProjectDefinition>,
}

pub struct ProjectDefinition {
    /// The name of the project. Name of the actual executable or library
    pub name: String,
    pub kind: ProjectKind,
    pub template: ProjectTemplate,
}

pub enum ProjectTemplate {
    RestService(RestServiceTemplate),
}

#[derive(Default)]
pub struct RestServiceTemplate {
    exclude_dependencies: Option<Vec<String>>,
    additional_dependencies: Option<Vec<Dependency>>,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum ProjectKind {
    #[default]
    Bin,
    Lib,
}

pub struct Dependency {
    /// The crate name
    pub name: String,
    /// The version of the crate
    pub version: Option<String>,
    /// The features to enable
    pub features: Vec<String>,
}

pub fn create_project(options: CreateOptions) -> anyhow::Result<()> {
    let root_dir = options
        .project_parent_dir
        .unwrap_or(std::env::current_dir()?);
    match options.project_structure {
        ProjectStructure::Single(project_definition) => {
            create_new_project(root_dir, project_definition)?;
        }
        ProjectStructure::Workspace(workspace_definition) => {
            create_workspace(&root_dir, &workspace_definition)?;
            let create_dir = root_dir.join(&workspace_definition.project_name);
            for pd in workspace_definition.projects {
                create_new_project(create_dir.clone(), pd)?;
            }
        }
    }

    Ok(())
}

mod workspace_cargo_toml {
    use serde::Serialize;

    #[derive(Serialize)]
    pub struct WorkspaceCargo {
        pub workspace: Workspace,
    }

    #[derive(Serialize)]
    pub struct Workspace {
        pub members: Vec<String>,
        pub resolver: Option<String>,
    }
}

fn create_workspace(
    root_dir: &PathBuf,
    workspace_definition: &WorkspaceDefinition,
) -> anyhow::Result<()> {
    let cargo_toml = workspace_cargo_toml::WorkspaceCargo {
        workspace: workspace_cargo_toml::Workspace {
            members: workspace_definition
                .projects
                .iter()
                .map(|p| p.name.clone())
                .collect(),
            resolver: Some("3".to_string()),
        },
    };
    let data = toml::to_string(&cargo_toml)?;

    std::fs::write(&root_dir, data)?;

    Ok(())
}

fn create_new_project(path: PathBuf, pd: ProjectDefinition) -> anyhow::Result<()> {
    let project_dir = path.join(&pd.name);
    let opts = NewOptions {
        version_control: None,
        kind: if matches!(pd.kind, ProjectKind::Bin) {
            NewProjectKind::Bin
        } else {
            NewProjectKind::Lib
        },
        auto_detect_kind: false,
        path: project_dir,
        name: Some(pd.name),
        edition: None,
        registry: None,
    };

    cargo::ops::new(&opts, &GlobalContext::default()?)?;

    match pd.template {
        ProjectTemplate::RestService {
            exclude_dependencies,
            additional_dependencies,
        } => add_dependencies(&project_dir, exclude_dependencies, additional_dependencies)?,
    }
    Ok(())
}

fn add_dependencies(
    project_dir: &PathBuf,
    exclude_dependencies: Option<Vec<String>>,
    additional_dependencies: Option<Vec<Dependency>>,
) -> anyhow::Result<()> {
    cargo::ops::cargo_add::add(workspace, options)
}
