//! Helpers for building out a project's directory/file structure and for
//! poking edits into files that already exist (e.g. wiring a new module
//! into `main.rs`).
//!
//! This is the toolkit `Template::scaffold` implementations use. It's kept
//! deliberately dumb/generic so you can change *what* gets scaffolded
//! without touching *how* scaffolding works.

use crate::error::OraeError;
use std::fs;
use std::path::{Path, PathBuf};

pub type Result<T> = std::result::Result<T, OraeError>;

/// Root handle for scaffolding operations, rooted at the generated
/// project's directory (i.e. the directory `cargo new` just created).
pub struct Scaffolder {
    root: PathBuf,
}

impl Scaffolder {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Resolve a path relative to the project root, e.g. `"src/routes/mod.rs"`.
    pub fn resolve(&self, relative_path: impl AsRef<Path>) -> PathBuf {
        self.root.join(relative_path)
    }

    /// Create an empty directory (and parents) relative to the project root.
    /// Does nothing if it already exists.
    pub fn create_dir(&self, relative_path: impl AsRef<Path>) -> Result<PathBuf> {
        let path = self.resolve(relative_path);
        fs::create_dir_all(&path).map_err(|source| OraeError::Io {
            path: path.clone(),
            source,
        })?;
        Ok(path)
    }

    /// Write a brand new file relative to the project root, creating parent
    /// directories as needed. Errors if the file already exists -- use
    /// [`Scaffolder::overwrite_file`] or [`Scaffolder::append_to_file`] if
    /// that's what you want instead.
    pub fn write_file(&self, relative_path: impl AsRef<Path>, contents: impl AsRef<str>) -> Result<PathBuf> {
        let path = self.resolve(relative_path);
        if path.exists() {
            return Err(OraeError::FileAlreadyExists(path));
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| OraeError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        fs::write(&path, contents.as_ref()).map_err(|source| OraeError::Io {
            path: path.clone(),
            source,
        })?;
        Ok(path)
    }

    /// Write a file relative to the project root, overwriting it if present.
    pub fn overwrite_file(&self, relative_path: impl AsRef<Path>, contents: impl AsRef<str>) -> Result<PathBuf> {
        let path = self.resolve(relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| OraeError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        fs::write(&path, contents.as_ref()).map_err(|source| OraeError::Io {
            path: path.clone(),
            source,
        })?;
        Ok(path)
    }

    /// Append text to the end of an existing (or new) file.
    pub fn append_to_file(&self, relative_path: impl AsRef<Path>, contents: impl AsRef<str>) -> Result<()> {
        use std::io::Write;
        let path = self.resolve(relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| OraeError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|source| OraeError::Io {
                path: path.clone(),
                source,
            })?;
        file.write_all(contents.as_ref().as_bytes())
            .map_err(|source| OraeError::Io {
                path: path.clone(),
                source,
            })?;
        Ok(())
    }

    /// Insert `contents` immediately after the first line matching `marker`
    /// (exact substring match) in an existing file. Useful for things like
    /// injecting `mod routes;` under a `// --- modules ---` marker line in
    /// `main.rs`, or adding a new route registration.
    ///
    /// Returns an error if the marker isn't found, so template authors
    /// notice immediately if a generated file's shape has drifted.
    pub fn insert_after_marker(
        &self,
        relative_path: impl AsRef<Path>,
        marker: &str,
        contents: &str,
    ) -> Result<()> {
        let path = self.resolve(relative_path);
        let existing = fs::read_to_string(&path).map_err(|source| OraeError::Io {
            path: path.clone(),
            source,
        })?;

        let mut found = false;
        let mut out = String::with_capacity(existing.len() + contents.len());
        for line in existing.lines() {
            out.push_str(line);
            out.push('\n');
            if !found && line.contains(marker) {
                out.push_str(contents);
                if !contents.ends_with('\n') {
                    out.push('\n');
                }
                found = true;
            }
        }

        if !found {
            return Err(OraeError::MarkerNotFound {
                path,
                marker: marker.to_string(),
            });
        }

        fs::write(&path, out).map_err(|source| OraeError::Io { path, source })
    }

    /// Start building a Rust module (a directory + `mod.rs`) at the given
    /// path segments, e.g. `scaffolder.module(&["src", "routes"])`.
    pub fn module<'a>(&'a self, path_segments: &[&str]) -> ModuleBuilder<'a> {
        let mut path = PathBuf::new();
        for segment in path_segments {
            path.push(segment);
        }
        ModuleBuilder {
            scaffolder: self,
            dir: path,
            mod_rs_contents: String::new(),
            submodules: Vec::new(),
        }
    }
}

/// Builder for a single Rust module directory (a directory containing a
/// `mod.rs`, optionally declaring submodules).
///
/// ```ignore
/// scaffolder
///     .module(&["src", "routes"])
///     .declare_submodule("health")
///     .declare_submodule("users")
///     .with_contents("// shared route helpers go here\n")
///     .build()?;
/// ```
pub struct ModuleBuilder<'a> {
    scaffolder: &'a Scaffolder,
    dir: PathBuf,
    mod_rs_contents: String,
    submodules: Vec<String>,
}

impl<'a> ModuleBuilder<'a> {
    /// Declare a `pub mod <name>;` line in this module's `mod.rs` and note
    /// that the caller should also create `<name>.rs` or `<name>/mod.rs`
    /// (this builder doesn't assume which -- create it yourself via
    /// [`Scaffolder::write_file`] or a nested [`Scaffolder::module`] call).
    pub fn declare_submodule(mut self, name: impl Into<String>) -> Self {
        self.submodules.push(name.into());
        self
    }

    /// Set/override the body of `mod.rs`, placed *after* the auto-generated
    /// `pub mod ...;` declarations.
    pub fn with_contents(mut self, contents: impl Into<String>) -> Self {
        self.mod_rs_contents = contents.into();
        self
    }

    /// Create the directory and its `mod.rs`.
    pub fn build(self) -> Result<PathBuf> {
        self.scaffolder.create_dir(&self.dir)?;

        let mut mod_rs = String::new();
        for submodule in &self.submodules {
            mod_rs.push_str(&format!("pub mod {submodule};\n"));
        }
        if !self.submodules.is_empty() {
            mod_rs.push('\n');
        }
        mod_rs.push_str(&self.mod_rs_contents);

        let mod_rs_path = self.dir.join("mod.rs");
        self.scaffolder.overwrite_file(mod_rs_path, mod_rs)?;
        Ok(self.scaffolder.resolve(&self.dir))
    }
}
