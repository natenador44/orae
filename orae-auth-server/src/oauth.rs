use axum::{Form, response::Response};
use orae_http_server::{RouteHandler, post, responses::HandlerResult, router_group};
use serde::Deserialize;

#[router_group(
    path = "/oauth2",
    routes = []
)]
pub struct LoginRouterGroup;

#[derive(Deserialize)]
struct OAuthCreds {
    username: String,
    password: String,
}

#[post("/token")]
struct Token {
    creds: Form<OAuthCreds>,
}
impl RouteHandler for Token {
    type Ok = Response;

    async fn handle(self) -> HandlerResult<Self::Ok> {
        todo!()
    }
}
