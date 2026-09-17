//! One file per project type. Add more (e.g. `grpc_service.rs`,
//! `worker.rs`) as your `api-*` crate suite grows, and register their
//! templates in [`crate::template::TemplateRegistry::default`].

pub mod rest_api;
