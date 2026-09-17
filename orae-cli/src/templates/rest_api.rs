//! Templates for `--type=rest-api`.
//!
//! [`ExampleRestApiTemplate`] below is a working stub that demonstrates how
//! to use the `Dependency`/`DependencySet`/`Scaffolder` APIs. Replace its
//! contents with your real dependency list and directory layout, and/or add
//! sibling structs (`AxumSqlxTemplate`, `ActixDieselTemplate`, ...) for the
//! different flavors you want to offer -- each one shows up as a separate
//! choice in the "which rest-api template?" prompt.

use crate::cli::ProjectType;
use crate::dependency::{Dependency, DependencySet, OptionalDependency};
use crate::scaffold::Scaffolder;
use crate::template::{Result, Template};

pub struct RestCrudTemplate;
impl Template for RestCrudTemplate {
    fn id(&self) -> &str {
        "rest-crud-app"
    }

    fn name(&self) -> &str {
        "REST CRUD API"
    }

    fn description(&self) -> &str {
        "Create modules and offer dependencies for data access and supporting REST APIs"
    }

    fn project_type(&self) -> ProjectType {
        ProjectType::RestApi
    }

    fn scaffold(&self, scaffolder: &Scaffolder) -> Result<()> {
        scaffolder
            .module(&["src", "routing"])
            .with_contents("// Declare routes here, or create sub-modules for different routes")
            .build()?;

        scaffolder
            .module(&["src", "models"])
            .with_contents("// Declare DTOs and/or Database Entities here.")
            .build()?;

        scaffolder
            .module(&["src", "db"])
            .with_contents("// Declare data access code here")
            .build()?;

        scaffolder.write_file(
            "src/routes/state.rs",
            r#"
            #[derive(Clone)]
            pub struct AppState {}
            "#,
        )?;

        scaffolder.overwrite_file(
            "src/main.rs",
            r#"
            mod state;
            mod routing;
            mod models;
            mod db;

            #[nkd_http_server::main(
                state = state::AppState,
            )]
            async fn main(app_builder: AppBuilder<state::AppState>) {
                todo!("add controllers and state to builder");
            }
            "#,
        )?;

        Ok(())
    }

    fn dependencies(&self) -> DependencySet {
        DependencySet::new().with_optional([OptionalDependency::new(
            "PostgreSQL",
            "A crate for accessing a postgres database",
            Dependency::new("tokio-postgres").with_version("0.7"),
        )])
    }
}
