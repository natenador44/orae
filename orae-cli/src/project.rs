//! Orchestrates the `orae create` flow end to end:
//!
//! 1. Pick a template (or none) for the requested project type.
//! 2. Pick optional dependencies.
//! 3. Confirm whether to generate the opinionated structure.
//! 4. Actually create the project, add dependencies, and (maybe) scaffold.

use std::path::PathBuf;

use crate::cargo_ops::{
    DependencyWriter, ManifestDependencyWriter, ProcessCargoRunner, ProjectCreator,
};
use crate::cli::{CreateArgs, ProjectType};
use crate::dependency::{
    Dependency, DependencySet, DependencySource, OptionalDependency, OptionalFeature,
};
use crate::prompts::{self, prompt_optional_dependencies};
use crate::scaffold::Scaffolder;
use crate::template::{Template, TemplateRegistry};
use anyhow::{Context, Result};
use itertools::Itertools;

pub fn run_create(args: CreateArgs) -> Result<()> {
    let project_name = args.name;

    let potential_dependencies = determine_potential_project_dependencies(args.project_type)?;

    let dependencies = gather_dependencies(potential_dependencies)?;

    // Step 4: create the project.
    let creator = ProcessCargoRunner;
    let writer = ManifestDependencyWriter;

    println!("Creating project {}", project_name);
    let project_dir = creator
        .create_binary_project(&args.path, &project_name)
        .with_context(|| format!("failed to create cargo project '{project_name}'"))?;

    println!("\nCreated `{project_name}` at {}", project_dir.display());

    let manifest_path = project_dir.join("Cargo.toml");

    writer
        .add_dependencies(&manifest_path, &dependencies)
        .context("failed to add dependencies to Cargo.toml")?;

    if !dependencies.is_empty() {
        println!(
            "Added dependencies: {}",
            dependencies
                .iter()
                .map(|d| d.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    let template_registry = TemplateRegistry::default();
    if let Some(template) = prompt_template_choice(&template_registry, args.project_type)? {
        let potential_template_dependencies = template.dependencies();
        println!(
            "Template {} comes with its own dependency options, prompting..",
            template.id()
        );
        let template_dependencies = gather_dependencies(potential_template_dependencies)?;
        writer
            .add_dependencies(&manifest_path, &template_dependencies)
            .context("failed to add dependencies to Cargo.toml")?;

        generate_project_structure_from_template(template, project_dir)?;

        println!("Created project structure from {} template", template.id());
    }

    Ok(())
}

fn gather_dependencies(mut potential_dependencies: DependencySet) -> Result<Vec<Dependency>> {
    let chosen_optional_dependencies =
        prompts::prompt_optional_dependencies(potential_dependencies.optional)?
            .into_iter()
            .map(|od| od.dependency);

    potential_dependencies
        .required
        .extend(chosen_optional_dependencies);

    let dependencies = prompts::prompt_optional_features(potential_dependencies.required)?;
    Ok(dependencies)
}

fn prompt_template_choice(
    template_registry: &TemplateRegistry,
    project_type: ProjectType,
) -> anyhow::Result<Option<&dyn Template>> {
    if prompts::prompt_use_template()? {
        let available_templates = template_registry.for_project_type(project_type);

        prompts::prompt_template_choice(&available_templates)
    } else {
        Ok(None)
    }
}

fn generate_project_structure_from_template(
    template: &dyn Template,
    project_dir: PathBuf,
) -> anyhow::Result<()> {
    let scaffolder = Scaffolder::new(&project_dir);
    template
        .scaffold(&scaffolder)
        .context("failed to generate project structure")?;
    Ok(())
}

fn determine_potential_project_dependencies(
    project_type: ProjectType,
) -> anyhow::Result<DependencySet> {
    let dependency_set = match project_type {
        ProjectType::RestApi => DependencySet::new()
            .with_required([
                Dependency::new("tokio")
                    .with_version("1")
                    .with_required_features(["full", "rt-multi-thread"]),
                Dependency::new("axum")
                    .with_version("0.8")
                    .with_required_features(["macros", "original-uri"])
                    .with_optional_features([
                        OptionalFeature::new(
                            "json",
                            "Support for JSON Request/Response Content Type",
                        ),
                        OptionalFeature::new("form", "Support for Form Request Content Type"),
                    ]),
                Dependency::new("tracing").with_version("0.1"),
                Dependency::new("orae-http-server").with_version("0.1")
                    .with_source(DependencySource::git("https://github.com/natenador44/orae.git")),
            ])
            .with_optional(
                [OptionalDependency::new(
                    "Serde",
                    "Provides the Serialize and Deserialize macros to use on your types",
                    Dependency::new("serde")
                        .with_version("1")
                        .with_required_features(["derive"]),
                ),
                OptionalDependency::new(
                    "JSON Parsing",
                    "Provides a crate for manual JSON serialization/deserialize (beyond request bodies)",
                    Dependency::new("serde_json").with_version("1"),
                ),]
            ),
    };

    Ok(dependency_set)
}
