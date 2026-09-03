use crate::{
    args::{Action::CreateProject, Args},
    create::{
        CreateOptions, ProjectDefinition, ProjectKind, ProjectStructure, ProjectTemplate,
        RestServiceTemplate,
    },
};

mod args;

mod create;

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.action {
        CreateProject { name, kind } => {
            let opts = CreateOptions {
                project_parent_dir: None,
                project_structure: ProjectStructure::Single(ProjectDefinition {
                    name,
                    kind,
                    template: ProjectTemplate::RestService(RestServiceTemplate::default()),
                }),
            };

            create::create_project(opts)?;
        }
    }

    Ok(())
}
