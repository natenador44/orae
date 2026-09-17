use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum OraeError {
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("file already exists: {0}")]
    FileAlreadyExists(PathBuf),

    #[error("marker '{marker}' not found in {path}")]
    MarkerNotFound { path: PathBuf, marker: String },

    #[error("`cargo {0}` failed (exit code {1:?}):\n{2}")]
    CargoCommandFailed(String, Option<i32>, String),

    #[error("failed to parse Cargo.toml at {path}: {source}")]
    ManifestParse {
        path: PathBuf,
        #[source]
        source: toml_edit::TomlError,
    },

    #[error("no template selected but scaffolding was requested")]
    ScaffoldRequestedWithoutTemplate,
}
