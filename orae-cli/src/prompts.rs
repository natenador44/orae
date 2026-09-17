//! All interactive prompting lives here, so the orchestration in
//! `project.rs` stays readable and the prompt *wording*/UX is easy to
//! tweak in one place.

use crate::dependency::{Dependency, OptionalDependency};
use crate::template::Template;
use anyhow::{Context, Result};
use inquire::{Confirm, MultiSelect, Select, Text};

/// Confirm (or let the user override) the project name. Defaults to
/// whatever was passed on the command line.
pub fn prompt_project_name(default: &str) -> Result<String> {
    Text::new("Project name:")
        .with_default(default)
        .prompt()
        .context("failed to read project name")
}

/// Step 2: "list out `rest-api` templates that are available, letting the
/// user choose one or none." Returns `None` if the user picks "None (plain
/// cargo project)".
pub fn prompt_template_choice<'a>(
    templates: &[&'a dyn Template],
) -> Result<Option<&'a dyn Template>> {
    if templates.is_empty() {
        return Ok(None);
    }

    const NONE_LABEL: &str = "None (plain cargo project)";

    let mut options: Vec<String> = templates
        .iter()
        .map(|t| format!("{} - {}", t.name(), t.description()))
        .collect();
    options.push(NONE_LABEL.to_string());

    let choice = Select::new("Choose a template:", options)
        .prompt()
        .context("failed to read template choice")?;

    if choice == NONE_LABEL {
        return Ok(None);
    }

    let index = templates
        .iter()
        .position(|t| choice.starts_with(t.name()))
        .expect("selected choice must match a known template");

    Ok(Some(templates[index]))
}

pub fn prompt_optional_features(mut for_dependencies: Vec<Dependency>) -> Result<Vec<Dependency>> {
    for dep in &mut for_dependencies {
        if !dep.optional_features.is_empty() {
            let optional_features = std::mem::take(&mut dep.optional_features);

            let selected_features = MultiSelect::new(
                &format!(
                    "Crate {} has optional dependencies, select which you want.",
                    dep.name
                ),
                optional_features,
            )
            .prompt()?
            .into_iter()
            .map(|of| of.name);

            dep.required_features.extend(selected_features);
        }
    }

    Ok(for_dependencies)
}

/// Step 3: "list optional dependencies/concepts and let the user choose
/// however many they like." Returns the subset the user selected, in the
/// order they were offered.
pub fn prompt_optional_dependencies(
    optional: Vec<OptionalDependency>,
) -> Result<Vec<OptionalDependency>> {
    if optional.is_empty() {
        return Ok(Vec::new());
    }

    let selected = MultiSelect::new("Select optional dependencies:", optional)
        .prompt()
        .context("failed to read optional dependency selection")?;

    Ok(selected)
}

/// The "yes/no" for whether to generate the opinionated project structure.
pub fn prompt_use_template() -> Result<bool> {
    Confirm::new("Generate the opinionated project structure for this template?")
        .with_default(true)
        .prompt()
        .context("failed to read structure confirmation")
}
