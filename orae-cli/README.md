# orae

An opinionated project generator for your `api-*` crate suite — a "Spring
Initializr" for your own Rust projects.

```
orae create --type=rest-api my-project
```

This walks you through:
1. Picking a template registered for `rest-api` (or "None" for a plain `cargo new`).
2. Picking optional dependencies via a multiselect.
3. Confirming whether to generate the opinionated directory structure.
4. Creating the project, writing dependencies into `Cargo.toml`, and (if
   confirmed) scaffolding the structure.

Verified to compile and run end-to-end against real crates.io dependencies.

## Where to make it yours

Nothing about *which* dependencies or *what* structure is baked in — that's
intentionally left for you to fill in:

- **`src/templates/rest_api.rs`** — replace `ExampleRestApiTemplate` with your
  real template(s). Add more `impl Template` structs here (or new files
  under `src/templates/` + a line in `src/templates/mod.rs`) for different
  flavors (e.g. axum vs actix), each becomes its own choice in step 1.
- **`src/template.rs`** — `TemplateRegistry::default()` is where templates
  get registered. Add `.register(...)` calls for each one.
- **`src/dependency.rs`** — `Dependency`, `DependencySource` (crates.io /
  git / path), `OptionalDependency`, `DependencySet`. Build these with the
  `with_*` builder methods.
- **`src/scaffold.rs`** — `Scaffolder` (create dirs/files, overwrite,
  append, insert-after-marker) and `ModuleBuilder` (directory + `mod.rs`
  with `pub mod` declarations) are the toolkit for `Template::scaffold`.
- **`src/cli.rs`** — `ProjectType` enum. Add a variant per project category
  (e.g. `GrpcService`) as your suite grows.

## Module map

| File | Responsibility |
|---|---|
| `main.rs` | Entry point, wires clap parsing to `project::run_create` |
| `cli.rs` | `clap` argument/subcommand definitions |
| `prompts.rs` | All `inquire` prompt wording/UX |
| `template.rs` | `Template` trait + `TemplateRegistry` |
| `templates/rest_api.rs` | Concrete templates for `--type=rest-api` |
| `dependency.rs` | `Dependency`/`DependencySource`/`DependencySet` model |
| `cargo_ops.rs` | `ProjectCreator` (shells to `cargo new`) + `DependencyWriter` (edits `Cargo.toml` via `toml_edit`) |
| `scaffold.rs` | `Scaffolder`/`ModuleBuilder` — dir/file creation helpers |
| `project.rs` | Orchestrates the full `create` flow |
| `error.rs` | `OraeError` |

## On "cargo as a library"

The `cargo` crate on crates.io is the actual `cargo` binary's internals —
not a stable embeddable API, and it changes with every Rust release. Tools
like `cargo-generate` shell out to the `cargo` binary and edit `Cargo.toml`
directly instead, which is what `cargo_ops.rs` does:

- `ProcessCargoRunner` shells out to `cargo new` (project scaffolding is
  simple enough that reimplementing it isn't worth it).
- `ManifestDependencyWriter` edits `Cargo.toml` directly with `toml_edit`
  (format-preserving), rather than shelling out to `cargo add` once per
  dependency. This also gives full control over path/git dependencies,
  which you'll want for your own in-development `api-*` crates.

Both are hidden behind traits (`ProjectCreator`, `DependencyWriter`) so you
can swap implementations later without touching `project.rs`.

## Try it

```
cargo run -- create --type=rest-api my-project
```
