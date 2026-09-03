use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{
    Expr, ExprArray, Field, Fields, FieldsNamed, ItemFn, ItemStruct, LitStr, Meta, MetaNameValue,
    Path, Token, Type, parse::Parser, parse_macro_input, punctuated::Punctuated, spanned::Spanned,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns the value `Expr` for a given key, or `None`.
fn find_key_value(args: &[(String, Expr)], key: &str) -> Option<Expr> {
    args.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
}

/// Parses `key = [item, item, ...]` and returns the items as a `Vec<Path>`.
fn parse_path_array(expr: &Expr) -> syn::Result<Vec<Path>> {
    match expr {
        Expr::Array(ExprArray { elems, .. }) => elems
            .iter()
            .map(|e| match e {
                Expr::Path(ep) => Ok(ep.path.clone()),
                other => Err(syn::Error::new(other.span(), "expected a path identifier")),
            })
            .collect(),
        other => Err(syn::Error::new(other.span(), "expected an array `[...]`")),
    }
}

/// Parses a comma-separated list of `ident = expr` pairs from a raw token stream.
fn parse_kv_args(input: proc_macro2::TokenStream) -> syn::Result<Vec<(String, Expr)>> {
    type KvPairs = Punctuated<syn::MetaNameValue, Token![,]>;
    let pairs = KvPairs::parse_terminated.parse2(input)?;
    pairs
        .into_iter()
        .map(|nv| {
            let key = nv
                .path
                .get_ident()
                .ok_or_else(|| syn::Error::new(nv.path.span(), "expected a simple identifier"))?
                .to_string();
            Ok((key, nv.value))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// #[nkd_http_server::main]
// ---------------------------------------------------------------------------
//
// Usage:
//
//   #[nkd_http_server::main(
//       state = AppState,
//       controllers = [DeviceController, ...]
//   )]
//   async fn main(app_builder: AppBuilder<AppState>) { ... }
//
// Generates:
//
//   type __AppState = AppState;
//
//   fn main() {
//       ::tokio::runtime::Builder::new_multi_thread()
//           .enable_all()
//           .build()
//           .unwrap()
//           .block_on(async {
//               let mut app_builder = AppBuilder::<AppState>::new();
//               app_builder = app_builder.controller(DeviceController);
//               __nkd_main(app_builder).await
//           });
//
//       async fn __nkd_main(app_builder: AppBuilder<AppState>) { /* user body */ }
//   }

#[proc_macro_attribute]
pub fn main(args: TokenStream, input: TokenStream) -> TokenStream {
    let args2 = proc_macro2::TokenStream::from(args);
    let item_fn = parse_macro_input!(input as ItemFn);

    let kv = match parse_kv_args(args2) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };

    // --- Extract `state = SomeType` ---
    let state_expr = match find_key_value(&kv, "state") {
        Some(e) => e,
        None => {
            return syn::Error::new(
                proc_macro2::Span::call_site(),
                "#[nkd_http_server::main] requires `state = <Type>`",
            )
            .to_compile_error()
            .into();
        }
    };
    // Re-parse the expr as a Type. Since `state = AppState` parses `AppState`
    // as an Expr::Path, round-tripping through quote works here.
    let state_type: Type = match syn::parse2(quote! { #state_expr }) {
        Ok(t) => t,
        Err(e) => return e.to_compile_error().into(),
    };

    // --- Validate the annotated function ---
    let fn_sig = &item_fn.sig;
    if fn_sig.asyncness.is_none() {
        return syn::Error::new(
            fn_sig.span(),
            "the function annotated with #[nkd_http_server::main] must be `async`",
        )
        .to_compile_error()
        .into();
    }
    let fn_body = &item_fn.block;
    let fn_inputs = &fn_sig.inputs;

    let inner_fn_name = format_ident!("__nkd_main");

    let expanded = quote! {
        // Expose the app-state type at crate root so controller/route macros
        // can reference it as `crate::__AppState`.
        type __AppState = #state_type;

        fn main() {
            ::nkd_http_server::setup_logging();
            ::tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    let mut app_builder =
                        ::nkd_http_server::AppBuilder::<#state_type>::new();
                    #inner_fn_name(app_builder).await
                });

            async fn #inner_fn_name(#fn_inputs) #fn_body
        }
    };

    expanded.into()
}

// ---------------------------------------------------------------------------
// #[nkd_http_server::controller]
// ---------------------------------------------------------------------------
//
// Usage:
//
//   #[nkd_http_server::controller(
//       context = "/device",
//       routes = [GetDevice, CreateDevice]
//   )]
//   pub struct DeviceController;
//
// Generates:
//
//   pub struct DeviceController;
//
//   impl Controller for DeviceController {
//       type State = crate::__AppState;
//       fn context() -> &'static str { "/device" }
//       fn router(&self) -> Router<Self::State> {
//           let mut router = Router::new();
//           router = <GetDevice as Route>::init(router);
//           router = <CreateDevice as Route>::init(router);
//           router
//       }
//   }

#[proc_macro_attribute]
pub fn controller(args: TokenStream, input: TokenStream) -> TokenStream {
    let args2 = proc_macro2::TokenStream::from(args);
    let item_struct = parse_macro_input!(input as ItemStruct);

    let kv = match parse_kv_args(args2) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };

    // --- Extract `context = "/some/path"` ---
    let context_expr = find_key_value(&kv, "context");

    let context_lit: LitStr = match context_expr {
        Some(e) => match syn::parse2(quote! { #e }) {
            Ok(l) => l,
            Err(_) => {
                return syn::Error::new(
                    e.span(),
                    "context must be a string literal, e.g. `context = \"/device\"`",
                )
                .to_compile_error()
                .into();
            }
        },
        None => LitStr::new("", Span::call_site()),
    };

    // --- Extract `routes = [...]` ---
    let routes_expr = match find_key_value(&kv, "routes") {
        Some(e) => e,
        None => {
            return syn::Error::new(
                proc_macro2::Span::call_site(),
                "#[nkd_http_server::controller] requires `routes = [...]`",
            )
            .to_compile_error()
            .into();
        }
    };
    let routes = match parse_path_array(&routes_expr) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };

    let struct_name = &item_struct.ident;
    let vis = &item_struct.vis;
    let attrs = &item_struct.attrs;

    let route_inits: Vec<TokenStream2> = routes
        .iter()
        .map(|r| {
            quote! {
                router = <#r as ::nkd_http_server::Route>::init(router);
                ::tracing::debug!(
                    http.method = <#r as ::nkd_http_server::Route>::method(),
                    uri = ::std::format!("{}{}", <Self as ::nkd_http_server::Controller>::context(), <#r as ::nkd_http_server::Route>::path()),
                    "registered route"
                );
            }
        })
        .collect();

    let expanded = quote! {
        // Re-emit the original struct unchanged.
        #(#attrs)*
        #vis struct #struct_name;

        impl ::nkd_http_server::Controller for #struct_name {
            type State = crate::__AppState;

            fn context() -> &'static str {
                #context_lit
            }

            fn router(&self) -> ::axum::Router<Self::State> {
                let mut router = ::axum::Router::new();
                #(#route_inits)*
                router
            }
        }
    };

    expanded.into()
}

// ---------------------------------------------------------------------------
// Shared route-macro logic
// ---------------------------------------------------------------------------

struct RouteField {
    kind: FieldKind,
    ident: syn::Ident,
    ty: Type,
}

#[derive(Debug, Eq)]
enum FieldKind {
    State,
    Path { name: Option<String> }, // name = explicit path variable name, if given
    QueryParams,
    RequestBody,
    Context,
}

impl PartialEq for FieldKind {
    fn eq(&self, other: &Self) -> bool {
        core::mem::discriminant(self) == core::mem::discriminant(other)
    }
}

impl PartialOrd for FieldKind {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FieldKind {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Self::RequestBody, _) => std::cmp::Ordering::Greater,
            (_, Self::RequestBody) => std::cmp::Ordering::Less,
            (_, _) => std::cmp::Ordering::Equal,
        }
    }
}

fn classify_field(field: &Field) -> syn::Result<RouteField> {
    let ident = field
        .ident
        .clone()
        .ok_or_else(|| syn::Error::new(field.span(), "route struct fields must be named"))?;

    let mut kind: Option<FieldKind> = None;
    for attr in &field.attrs {
        let path = attr.path();
        if path.is_ident("state") {
            kind = Some(FieldKind::State);
        } else if path.is_ident("path") {
            // #[path] or #[path(name = "some_name")]
            let explicit_name = match &attr.meta {
                // bare #[path] with no arguments
                Meta::Path(_) => None,
                // #[path(name = "...")]
                Meta::List(list) => {
                    let nv: MetaNameValue = list.parse_args()?;
                    if !nv.path.is_ident("name") {
                        return Err(syn::Error::new(
                            nv.path.span(),
                            "the only supported argument to #[path] is `name = \"...\"`",
                        ));
                    }
                    match &nv.value {
                        Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Str(s),
                            ..
                        }) => Some(s.value()),
                        other => {
                            return Err(syn::Error::new(
                                other.span(),
                                "#[path(name = ...)] value must be a string literal",
                            ));
                        }
                    }
                }
                Meta::NameValue(_) => {
                    return Err(syn::Error::new(
                        attr.span(),
                        "use #[path(name = \"...\")] not #[path = \"...\"]",
                    ));
                }
            };
            kind = Some(FieldKind::Path {
                name: explicit_name,
            });
        } else if path.is_ident("query_params") {
            kind = Some(FieldKind::QueryParams);
        } else if path.is_ident("request_body") {
            kind = Some(FieldKind::RequestBody);
        } else if path.is_ident("context") {
            kind = Some(FieldKind::Context)
        }
    }

    let kind = kind.ok_or_else(|| {
        syn::Error::new(
            field.span(),
            "every field in a route struct must carry one of: \
             #[state], #[path], #[query_params], #[request_body], #[context]",
        )
    })?;

    Ok(RouteField {
        kind,
        ident,
        ty: field.ty.clone(),
    })
}

/// Returns a copy of the fields with our helper attributes stripped so the
/// re-emitted struct doesn't carry unknown attributes.
fn strip_route_attrs(fields: &FieldsNamed) -> Vec<Field> {
    fields
        .named
        .iter()
        .map(|f| {
            let mut f = f.clone();
            f.attrs.retain(|a| {
                !a.path().is_ident("state")
                    && !a.path().is_ident("path")
                    && !a.path().is_ident("query_params")
                    && !a.path().is_ident("request_body")
                    && !a.path().is_ident("context")
            });
            f
        })
        .collect()
}

/// Core expansion shared by get/post/put/delete/patch.
///
/// `method_fn` is the `axum::routing::*` token, e.g. `quote!{ ::axum::routing::get }`.
fn expand_route_macro(
    path_lit: LitStr,
    item_struct: ItemStruct,
    method_fn: TokenStream2,
    method_name: &str,
) -> syn::Result<TokenStream2> {
    let struct_name = &item_struct.ident;
    let vis = &item_struct.vis;

    let named_fields = match &item_struct.fields {
        Fields::Named(n) => n,
        _ => {
            return Err(syn::Error::new(
                item_struct.fields.span(),
                "route structs must have named fields",
            ));
        }
    };

    let mut classified: Vec<RouteField> = named_fields
        .named
        .iter()
        .map(classify_field)
        .collect::<syn::Result<_>>()?;

    classified.sort_by(|l, r| l.kind.cmp(&r.kind));

    // Validate cardinality: at most one #[query_params] and #[request_body].
    // Multiple #[path] and #[state] fields are now allowed.
    let mut seen_query = false;
    let mut seen_body = false;
    let mut seen_ctx = false;
    for rf in &classified {
        match rf.kind {
            FieldKind::QueryParams => {
                if seen_query {
                    return Err(syn::Error::new(
                        proc_macro2::Span::call_site(),
                        "at most one #[query_params] field is allowed per route struct",
                    ));
                }
                seen_query = true;
            }
            FieldKind::RequestBody => {
                if seen_body {
                    return Err(syn::Error::new(
                        proc_macro2::Span::call_site(),
                        "at most one #[request_body] field is allowed per route struct",
                    ));
                }
                seen_body = true;
            }
            FieldKind::Path { .. } | FieldKind::State => {}
            FieldKind::Context => {
                if seen_ctx {
                    return Err(syn::Error::new(
                        proc_macro2::Span::call_site(),
                        "at most one #[context] field is allowed per route struct",
                    ));
                }
                seen_ctx = true;
            }
        }
    }

    // Collect all path fields and validate their `name` values appear in the
    // route template. Axum matches path variables by tuple position, so the
    // `name` doesn't affect the extractor — but catching mismatches at
    // compile time is much friendlier than a runtime 500.
    let route_template = path_lit.value();
    let path_fields: Vec<&RouteField> = classified
        .iter()
        .filter(|rf| matches!(rf.kind, FieldKind::Path { .. }))
        .collect();

    for rf in &path_fields {
        if let FieldKind::Path {
            name: Some(explicit_name),
            ..
        } = &rf.kind
        {
            let placeholder = format!("{{{}}}", explicit_name);
            if !route_template.contains(&placeholder) {
                return Err(syn::Error::new(
                    rf.ident.span(),
                    format!(
                        "path variable `{}` not found in route template `{}`; \
                         check the `name` argument matches a `{{param}}` in the path",
                        explicit_name, route_template
                    ),
                ));
            }
        } else if let FieldKind::Path { name: None } = rf.kind {
            // No explicit name: the field name itself must appear in the template.
            let placeholder = format!("{{{}}}", rf.ident);
            if !route_template.contains(&placeholder) {
                return Err(syn::Error::new(
                    rf.ident.span(),
                    format!(
                        "field `{}` has #[path] but `{{{}}}` was not found in route \
                         template `{}`; add `name = \"param_name\"` if the path \
                         variable has a different name",
                        rf.ident, rf.ident, route_template
                    ),
                ));
            }
        }
    }

    // Build handler params and struct constructor assignments.
    //
    // Path fields are special: if there are multiple, they collapse into a
    // single Path<(T1, T2, ...)> extractor that is then destructured.
    //
    //   single:   Path(device_id): Path<u64>
    //   multiple: Path((project_name, branch, file_path)): Path<(String, String, String)>

    let mut handler_params: Vec<TokenStream2> = Vec::new();
    let mut struct_field_assignments: Vec<TokenStream2> = Vec::new();

    // Emit the Path extractor first (axum requires Path before State).
    if !path_fields.is_empty() {
        let path_idents: Vec<&syn::Ident> = path_fields.iter().map(|rf| &rf.ident).collect();
        let path_types: Vec<&Type> = path_fields.iter().map(|rf| &rf.ty).collect();

        if path_fields.len() == 1 {
            let ident = path_idents[0];
            let ty = path_types[0];
            handler_params.push(quote! {
                ::axum::extract::Path(#ident): ::axum::extract::Path<#ty>
            });
        } else {
            handler_params.push(quote! {
                ::axum::extract::Path((#(#path_idents),*)): ::axum::extract::Path<(#(#path_types),*)>
            });
        }
        for ident in &path_idents {
            struct_field_assignments.push(quote! { #ident });
        }
    }

    // Emit remaining extractors in the order they appear in the struct.
    for rf in &classified {
        let ident = &rf.ident;
        let ty = &rf.ty;
        match &rf.kind {
            FieldKind::Path { .. } => {
                // Already handled above; skip here to avoid duplicates.
            }
            FieldKind::State => {
                handler_params.push(quote! {
                    ::axum::extract::State(#ident): ::axum::extract::State<#ty>
                });
                struct_field_assignments.push(quote! { #ident });
            }
            FieldKind::QueryParams => {
                handler_params.push(quote! {
                    ::axum::extract::Query(#ident): ::axum::extract::Query<#ty>
                });
                struct_field_assignments.push(quote! { #ident });
            }
            FieldKind::RequestBody => {
                handler_params.push(quote! {
                    ::axum::extract::Json(#ident): ::axum::extract::Json<#ty>
                });
                struct_field_assignments.push(quote! { #ident });
            }
            FieldKind::Context => {
                handler_params.push(quote! {
                    ::axum::extract::OriginalUri(uri): ::axum::extract::OriginalUri
                });
                struct_field_assignments.push(quote! {
                    #ident: ::nkd_http_server::context::RequestContext { uri: uri.to_string(), }
                });
            }
        }
    }

    let cleaned_fields = strip_route_attrs(named_fields);
    let struct_attrs: Vec<_> = item_struct
        .attrs
        .iter()
        .filter(|a| {
            !a.path().is_ident("get")
                && !a.path().is_ident("post")
                && !a.path().is_ident("put")
                && !a.path().is_ident("delete")
                && !a.path().is_ident("patch")
        })
        .collect();

    let handler_fn_name = format_ident!("{}_handler", struct_name);
    let route_path = &path_lit;
    let method_name_lit = method_name;

    let expanded = quote! {
        #(#struct_attrs)*
        #vis struct #struct_name {
            #(#cleaned_fields),*
        }

        impl ::nkd_http_server::Route for #struct_name {
            type State = crate::__AppState;

            fn init(router: ::axum::Router<Self::State>) -> ::axum::Router<Self::State> {
                router.route(#route_path, #method_fn(#handler_fn_name))
            }

            fn method() -> &'static str {
                #method_name_lit
            }

            fn path() -> &'static str {
                #route_path
            }
        }

        #[allow(non_snake_case)]
        async fn #handler_fn_name(
            #(#handler_params),*
        ) -> impl ::axum::response::IntoResponse {
            let result = #struct_name {
                #(#struct_field_assignments),*
            }
            .handle()
            .await;

            if let Err(e) = &result {
                ::tracing::error!("{:?}", e);
            }

            result
        }
    };

    Ok(expanded)
}

// ---------------------------------------------------------------------------
// Per-method attribute macros
// ---------------------------------------------------------------------------
//
// These cannot be generated with macro_rules! because #[proc_macro_attribute]
// and parse_macro_input! are not usable inside macro_rules!. Each is a thin
// wrapper that passes the right axum routing function token to the shared
// expand_route_macro helper.

#[proc_macro_attribute]
pub fn get(args: TokenStream, input: TokenStream) -> TokenStream {
    let path_lit = parse_macro_input!(args as LitStr);
    let item_struct = parse_macro_input!(input as ItemStruct);
    let method_fn = quote! { ::axum::routing::get };
    match expand_route_macro(path_lit, item_struct, method_fn, "GET") {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

#[proc_macro_attribute]
pub fn post(args: TokenStream, input: TokenStream) -> TokenStream {
    let path_lit = parse_macro_input!(args as LitStr);
    let item_struct = parse_macro_input!(input as ItemStruct);
    let method_fn = quote! { ::axum::routing::post };
    match expand_route_macro(path_lit, item_struct, method_fn, "POST") {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

#[proc_macro_attribute]
pub fn put(args: TokenStream, input: TokenStream) -> TokenStream {
    let path_lit = parse_macro_input!(args as LitStr);
    let item_struct = parse_macro_input!(input as ItemStruct);
    let method_fn = quote! { ::axum::routing::put };
    match expand_route_macro(path_lit, item_struct, method_fn, "PUT") {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

#[proc_macro_attribute]
pub fn delete(args: TokenStream, input: TokenStream) -> TokenStream {
    let path_lit = parse_macro_input!(args as LitStr);
    let item_struct = parse_macro_input!(input as ItemStruct);
    let method_fn = quote! { ::axum::routing::delete };
    match expand_route_macro(path_lit, item_struct, method_fn, "DELETE") {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

#[proc_macro_attribute]
pub fn patch(args: TokenStream, input: TokenStream) -> TokenStream {
    let path_lit = parse_macro_input!(args as LitStr);
    let item_struct = parse_macro_input!(input as ItemStruct);
    let method_fn = quote! { ::axum::routing::patch };
    match expand_route_macro(path_lit, item_struct, method_fn, "PATCH") {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

// ---------------------------------------------------------------------------
// Helper attribute stubs
// ---------------------------------------------------------------------------
//
// #[state], #[path], #[query_params], and #[request_body] are consumed and
// stripped by the route macros above when they process the enclosing struct.
// These stub declarations exist so that:
//
//   1. `rustc` doesn't reject the attributes as "unknown" during IDE
//      incremental analysis before the route macro runs.
//   2. Users get a sensible error if they accidentally apply one of these
//      to a bare item rather than a field inside a route struct.
//
// The stubs are intentional no-ops: they return the input token stream
// unchanged and let the outer route macro do all the real work.

#[proc_macro_attribute]
pub fn state(_args: TokenStream, input: TokenStream) -> TokenStream {
    input
}

#[proc_macro_attribute]
pub fn path(_args: TokenStream, input: TokenStream) -> TokenStream {
    input
}

#[proc_macro_attribute]
pub fn context(_args: TokenStream, input: TokenStream) -> TokenStream {
    input
}

#[proc_macro_attribute]
pub fn query_params(_args: TokenStream, input: TokenStream) -> TokenStream {
    input
}

#[proc_macro_attribute]
pub fn request_body(_args: TokenStream, input: TokenStream) -> TokenStream {
    input
}
