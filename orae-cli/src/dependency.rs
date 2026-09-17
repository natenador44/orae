//! Everything related to describing and adding Cargo dependencies.
//!
//! This is the part you'll edit most often: define the crates you want
//! `orae` to know about (your own `api-*` crates plus community crates)
//! by building [`Dependency`] values. Nothing here is opinionated about
//! *which* crates to use -- that lives in `templates/*.rs`.

/// Where a dependency comes from. Mirrors the sources `cargo add` supports.
#[derive(Debug, Clone)]
pub enum DependencySource {
    /// A normal crates.io dependency.
    CratesIo,
    /// A git dependency, e.g. for your own crates before they're published.
    Git {
        url: String,
        branch: Option<String>,
        tag: Option<String>,
        rev: Option<String>,
    },
}

impl DependencySource {
    pub fn git(url: impl Into<String>) -> Self {
        DependencySource::Git {
            url: url.into(),
            branch: None,
            tag: None,
            rev: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OptionalFeature {
    pub name: String,
    pub description: String,
}

impl OptionalFeature {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
        }
    }
}

impl std::fmt::Display for OptionalFeature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // inquire's MultiSelect uses Display for the list labels.
        write!(f, "{} - {}", self.name, self.description)
    }
}

/// A single dependency to add to the generated project's `Cargo.toml`.
///
/// Construct these with [`Dependency::new`] and the builder-style `with_*`
/// methods, e.g.:
///
/// ```ignore
/// Dependency::new("api-router")
///     .with_source(DependencySource::path("../api-router"))
///     .with_features(["axum"])
/// ```
///
/// TODO option to pull these from service or git repo
#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub version: Option<String>,
    pub required_features: Vec<String>,
    pub optional_features: Vec<OptionalFeature>,
    pub default_features: bool,
    pub dev: bool,
    pub source: DependencySource,
}

impl Dependency {
    /// A plain crates.io dependency with default settings.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: None,
            required_features: Default::default(),
            optional_features: Default::default(),
            default_features: true,
            dev: false,
            source: DependencySource::CratesIo,
        }
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    pub fn with_source(mut self, source: DependencySource) -> Self {
        self.source = source;
        self
    }

    pub fn with_required_features<S: Into<String>>(
        mut self,
        features: impl IntoIterator<Item = S>,
    ) -> Self {
        self.required_features = features.into_iter().map(|s| s.into()).collect();
        self
    }

    pub fn with_optional_features(
        mut self,
        features: impl IntoIterator<Item = OptionalFeature>,
    ) -> Self {
        self.optional_features = features.into_iter().collect();
        self
    }

    pub fn no_default_features(mut self) -> Self {
        self.default_features = false;
        self
    }

    pub fn as_dev_dependency(mut self) -> Self {
        self.dev = true;
        self
    }
}

/// A dependency that's presented to the user as an opt-in choice, e.g.
/// "add tracing?" or "add sqlx (postgres)?".
///
/// `label` and `description` are what get shown in the `inquire` multiselect;
/// `dependency` is what actually gets added if the user picks it.
#[derive(Debug, Clone)]
pub struct OptionalDependency {
    pub label: &'static str,
    pub description: &'static str,
    pub dependency: Dependency,
}

impl OptionalDependency {
    pub const fn new(
        label: &'static str,
        description: &'static str,
        dependency: Dependency,
    ) -> Self {
        Self {
            label,
            description,
            dependency,
        }
    }
}

impl std::fmt::Display for OptionalDependency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // inquire's MultiSelect uses Display for the list labels.
        write!(f, "{} - {}", self.label, self.description)
    }
}

/// The full set of dependencies a template contributes: required ones that
/// are always added, and optional ones the user picks from.
#[derive(Debug, Clone, Default)]
pub struct DependencySet {
    pub required: Vec<Dependency>,
    pub optional: Vec<OptionalDependency>,
}

impl DependencySet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_required(mut self, deps: impl IntoIterator<Item = Dependency>) -> Self {
        self.required.extend(deps);
        self
    }

    pub fn with_optional(mut self, deps: impl IntoIterator<Item = OptionalDependency>) -> Self {
        self.optional.extend(deps);
        self
    }
}
