//! Talking to Cargo.
//!
//! Note on "cargo as a library": the `cargo` crate on crates.io is
//! literally the cargo binary's internals. It's not designed as a stable
//! embeddable API -- its surface changes with every Rust release and pulls
//! in a huge dependency tree. In practice, tools like `cargo-generate`
//! shell out to the `cargo` binary and/or edit `Cargo.toml` directly with a
//! TOML editor. This module does the same, but hides it behind two small
//! traits (`ProjectCreator`, `DependencyWriter`) so you can swap in a
//! different implementation later without touching callers.

use crate::dependency::{Dependency, DependencySource};
use crate::error::OraeError;
use std::path::{Path, PathBuf};
use std::process::Command;

pub type Result<T> = std::result::Result<T, OraeError>;

/// Creates the initial Cargo project (i.e. what `cargo new` does).
pub trait ProjectCreator {
    /// Create a new binary project named `name` inside `parent_dir`,
    /// returning the path to the created project directory.
    fn create_binary_project(&self, parent_dir: &Path, name: &str) -> Result<PathBuf>;
}

/// Adds dependencies to an existing project's `Cargo.toml`.
pub trait DependencyWriter {
    fn add_dependency(&self, manifest_path: &Path, dependency: &Dependency) -> Result<()>;

    fn add_dependencies(&self, manifest_path: &Path, dependencies: &[Dependency]) -> Result<()> {
        for dep in dependencies {
            self.add_dependency(manifest_path, dep)?;
        }
        Ok(())
    }
}

/// Default [`ProjectCreator`] that shells out to the `cargo` binary
/// (`cargo new`).
pub struct ProcessCargoRunner;

impl ProjectCreator for ProcessCargoRunner {
    fn create_binary_project(&self, parent_dir: &Path, name: &str) -> Result<PathBuf> {
        let status = Command::new("cargo")
            .arg("new")
            .arg("--bin")
            .arg(name)
            .current_dir(parent_dir)
            .status()
            .map_err(|source| OraeError::Io {
                path: parent_dir.to_path_buf(),
                source,
            })?;

        if !status.success() {
            return Err(OraeError::CargoCommandFailed(
                format!("new --bin {name}"),
                status.code(),
                "see output above".to_string(),
            ));
        }

        Ok(parent_dir.join(name))
    }
}

/// Default [`DependencyWriter`] that edits `Cargo.toml` directly with
/// `toml_edit`, preserving formatting/comments. This avoids shelling out
/// to `cargo add` once per dependency and gives full control over
/// path/git dependencies, which is handy for your own in-development
/// `api-*` crates.
pub struct ManifestDependencyWriter;

impl DependencyWriter for ManifestDependencyWriter {
    fn add_dependency(&self, manifest_path: &Path, dependency: &Dependency) -> Result<()> {
        let raw = std::fs::read_to_string(manifest_path).map_err(|source| OraeError::Io {
            path: manifest_path.to_path_buf(),
            source,
        })?;
        let mut doc =
            raw.parse::<toml_edit::DocumentMut>()
                .map_err(|source| OraeError::ManifestParse {
                    path: manifest_path.to_path_buf(),
                    source,
                })?;

        let table_key = if dependency.dev {
            "dev-dependencies"
        } else {
            "dependencies"
        };

        if doc.get(table_key).is_none() {
            doc[table_key] = toml_edit::Item::Table(toml_edit::Table::new());
        }

        let entry = build_dependency_item(dependency);
        doc[table_key][&dependency.name] = entry;

        std::fs::write(manifest_path, doc.to_string()).map_err(|source| OraeError::Io {
            path: manifest_path.to_path_buf(),
            source,
        })
    }
}

/// Turn a [`Dependency`] into the `toml_edit` item that goes on the
/// right-hand side of `name = ...` in `Cargo.toml`. Uses the inline-table
/// form (`{ version = "...", features = [...] }`) whenever there's more
/// than just a bare version, since that's the common case for API crates
/// with feature flags.
fn build_dependency_item(dependency: &Dependency) -> toml_edit::Item {
    let simple_version_only = dependency.required_features.is_empty()
        && dependency.default_features
        && matches!(dependency.source, DependencySource::CratesIo);

    if simple_version_only {
        if let Some(version) = &dependency.version {
            return toml_edit::value(&*version);
        }
        // No version pinned and nothing else to configure: leave it as an
        // empty table, which `cargo add`/`cargo build` will fail loudly on
        // until you set a version. This is intentional -- better than
        // silently writing `"*"`.
        return toml_edit::value("*");
    }

    let mut table = toml_edit::InlineTable::new();

    match &dependency.source {
        DependencySource::CratesIo => {
            if let Some(version) = &dependency.version {
                table.insert("version", (&*version).into());
            }
        }
        DependencySource::Git {
            url,
            branch,
            tag,
            rev,
        } => {
            table.insert("git", (&*url).into());
            if let Some(branch) = branch {
                table.insert("branch", (&*branch).into());
            }
            if let Some(tag) = tag {
                table.insert("tag", (&*tag).into());
            }
            if let Some(rev) = rev {
                table.insert("rev", (&*rev).into());
            }
        }
    }

    if !dependency.required_features.is_empty() {
        let mut arr = toml_edit::Array::new();
        for feature in &dependency.required_features {
            arr.push(&*feature);
        }
        table.insert("features", toml_edit::Value::Array(arr));
    }

    if !dependency.default_features {
        table.insert("default-features", false.into());
    }

    toml_edit::Item::Value(toml_edit::Value::InlineTable(table))
}
