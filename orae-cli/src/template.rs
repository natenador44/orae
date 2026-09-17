//! The template system: each [`Template`] is one selectable "flavor" of a
//! given [`ProjectType`] (e.g. multiple takes on `rest-api`, like
//! "axum + sqlx" vs "actix-web + diesel"). This is the extension point --
//! add a new file under `templates/`, implement `Template`, and register it
//! in [`TemplateRegistry::default`].

use crate::cli::ProjectType;
use crate::dependency::DependencySet;
use crate::error::OraeError;
use crate::scaffold::Scaffolder;

pub type Result<T> = std::result::Result<T, OraeError>;

/// One selectable template for a [`ProjectType`].
///
/// Implementors describe:
/// - what dependencies are always required ([`Template::dependencies`]'s
///   `required`),
/// - what dependencies are offered as opt-in choices (`optional`), and
/// - how to lay out the project directory, if the user opts into
///   structure generation ([`Template::scaffold`]).
pub trait Template {
    /// Stable identifier, e.g. `"axum-minimal"`. Not shown to the user.
    fn id(&self) -> &str;

    /// Short human-readable name shown in the selection prompt.
    fn name(&self) -> &str;

    /// One-line description shown alongside the name.
    fn description(&self) -> &str;

    /// Which [`ProjectType`] this template applies to.
    fn project_type(&self) -> ProjectType;

    fn dependencies(&self) -> DependencySet;

    /// Build out the project's directory/file structure using the given
    /// [`Scaffolder`], rooted at the freshly-created project directory.
    /// Only called if the user says yes to structure generation.
    fn scaffold(&self, scaffolder: &Scaffolder) -> Result<()>;
}

impl std::fmt::Display for dyn Template {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {}", self.name(), self.description())
    }
}

/// Holds every known template. Populate this in `TemplateRegistry::default`
/// (or via `register`) as you build out your own templates.
pub struct TemplateRegistry {
    templates: Vec<Box<dyn Template>>,
}

impl TemplateRegistry {
    pub fn new() -> Self {
        Self {
            templates: Vec::new(),
        }
    }

    pub fn register(mut self, template: impl Template + 'static) -> Self {
        self.templates.push(Box::new(template));
        self
    }

    /// All templates registered for a given project type, in registration
    /// order. This is what gets shown in the "choose a template" prompt.
    pub fn for_project_type(&self, project_type: ProjectType) -> Vec<&dyn Template> {
        self.templates
            .iter()
            .filter(|t| t.project_type() == project_type)
            .map(|t| t.as_ref())
            .collect()
    }
}

impl Default for TemplateRegistry {
    fn default() -> Self {
        // Register your templates here, e.g.:
        //
        //   TemplateRegistry::new()
        //       .register(templates::rest_api::AxumMinimalTemplate)
        //
        TemplateRegistry::new().register(crate::templates::rest_api::RestCrudTemplate)
    }
}
