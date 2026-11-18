use orae_http_server::router_group;

#[router_group(
    path = "/login",
    routes = []
)]
pub struct LoginRouterGroup;
