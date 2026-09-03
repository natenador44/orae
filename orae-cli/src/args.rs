use clap::{
    Arg, Command, ValueEnum,
    builder::{EnumValueParser, PossibleValue},
    value_parser,
};

use crate::create::ProjectKind;

const CREATE_SUBCOMMAND: &str = "create";
const PROJECT_NAME_ARG: &str = "project_name";
const PROJECT_KIND_ARG: &str = "project_kind";

#[derive(Debug)]
pub struct Args {
    pub action: Action,
}

#[derive(Debug)]
pub enum Action {
    CreateProject { name: String, kind: ProjectKind },
}

impl Args {
    pub fn parse() -> Self {
        let matches = Command::new("nkdhs")
            .subcommand(create_project_subcommand())
            .get_matches();

        match matches.subcommand() {
            Some((CREATE_SUBCOMMAND, matches)) => Self {
                action: Action::CreateProject {
                    name: matches
                        .get_one(PROJECT_NAME_ARG)
                        .cloned()
                        .expect("project name required arg"),
                    kind: matches
                        .get_one(PROJECT_KIND_ARG)
                        .copied()
                        .unwrap_or_default(),
                },
            },
            _ => todo!(),
        }
    }
}

fn create_project_subcommand() -> Command {
    Command::new(CREATE_SUBCOMMAND)
        .arg(project_name_arg())
        .arg(project_kind_arg())
}

fn project_kind_arg() -> Arg {
    Arg::new(PROJECT_KIND_ARG)
        .help("The type of the project")
        .value_parser(value_parser!(ProjectKind))
}

impl ValueEnum for ProjectKind {
    fn value_variants<'a>() -> &'a [Self] {
        &[ProjectKind::Bin, ProjectKind::Lib]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        match self {
            ProjectKind::Bin => Some(PossibleValue::new("bin")),
            ProjectKind::Lib => Some(PossibleValue::new("lib")),
        }
    }
}

fn project_name_arg() -> Arg {
    Arg::new(PROJECT_NAME_ARG)
        .required(true)
        .help("The name of the project")
}
