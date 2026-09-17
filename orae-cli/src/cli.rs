use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    name = "orae",
    version,
    about = "Scaffold opinionated Rust API projects, built on your own crate suite"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Create a new project.
    Create(CreateArgs),
}

#[derive(clap::Args, Debug)]
pub struct CreateArgs {
    /// Name of the project (and the directory that will be created for it).
    pub name: String,

    /// Type of project to scaffold.
    #[arg(long = "type", value_enum)]
    pub project_type: ProjectType,

    /// Directory to create the project in. Defaults to the current directory.
    #[arg(long, default_value = ".")]
    pub path: std::path::PathBuf,
}

/// The kinds of projects `orae` knows how to bootstrap. Add a variant here
/// for each new category (e.g. `GrpcService`, `Worker`), then register
/// matching [`crate::template::Template`] implementations.
#[derive(Copy, Clone, Eq, PartialEq, Debug, ValueEnum)]
pub enum ProjectType {
    #[value(name = "rest-api")]
    RestApi,
}

impl std::fmt::Display for ProjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectType::RestApi => write!(f, "rest-api"),
        }
    }
}
