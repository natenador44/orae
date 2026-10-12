//! Derives for "one of several success responses" enums in axum.
//!
//! ```ignore
//! #[derive(IntoResponse, FromVariants)]
//! enum ListResponse {
//!     Empty(NoContent),
//!     Items(Json<Vec<String>>),
//! }
//!
//! async fn list() -> ListResponse {
//!     if empty { NoContent.into() } else { Json(items).into() }
//! }
//! ```
//!
//! Every variant must wrap exactly one value (tuple or single named field).
//! Bodyless responses are expressed by wrapping a unit struct such as
//! `NoContent`; anything else is a compile error:
//!
//! ```compile_fail
//! use axum_response_derive::IntoResponse;
//!
//! #[derive(IntoResponse)]
//! enum Bad {
//!     Nothing,
//! }
//! ```
//!
//! ```compile_fail
//! use axum_response_derive::IntoResponse;
//!
//! #[derive(IntoResponse)]
//! struct NotAnEnum(u8);
//! ```

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parse_macro_input, parse_quote_spanned, spanned::Spanned, Data, DeriveInput, Error, Fields,
    Ident, Result, Type,
};

/// One enum variant wrapping exactly one value.
struct Variant<'a> {
    ident: &'a Ident,
    /// `Some` for `V { name: T }`, `None` for `V(T)`.
    field: Option<&'a Ident>,
    ty: &'a Type,
    serialize_type: Option<SerializeType>,
}

enum SerializeType {
    Json
}

impl Variant<'_> {
    /// Pattern binding the wrapped value to `inner`.
    fn pattern(&self) -> TokenStream2 {
        let v = self.ident;
        match self.field {
            Some(f) => quote!(Self::#v { #f: inner }),
            None => quote!(Self::#v(inner)),
        }
    }

    /// Expression building the variant from `inner`.
    fn constructor(&self) -> TokenStream2 {
        let v = self.ident;
        match self.field {
            Some(f) => quote!(Self::#v { #f: inner }),
            None => quote!(Self::#v(inner)),
        }
    }
}

fn variants<'a>(input: &'a DeriveInput, derive: &str) -> Result<Vec<Variant<'a>>> {
    let Data::Enum(data) = &input.data else {
        return Err(Error::new_spanned(
            &input.ident,
            format!("`#[derive({derive})]` can only be used on enums"),
        ));
    };

    data.variants
        .iter()
        .map(|v| {
            let mut serialize_type = None;
            for attr in &v.attrs {
                if attr.path().is_ident("json") {
                    attr.meta.require_path_only()?;
                    serialize_type = Some(SerializeType::Json);
                }
            }
            let field = match &v.fields {
                Fields::Unnamed(f) if f.unnamed.len() == 1 => &f.unnamed[0],
                Fields::Named(f) if f.named.len() == 1 => &f.named[0],
                _ => {
                    return Err(Error::new_spanned(
                        v,
                        "each variant must wrap exactly one value, e.g. `Items(Json<Vec<String>>)`; \
                         for a bodyless response, wrap a unit struct like `NoContent`",
                    ))
                }
            };
            Ok(Variant {
                ident: &v.ident,
                field: field.ident.as_ref(),
                ty: &field.ty,
                serialize_type,
            })
        })
        .collect()
}

pub fn expand_response_variants(input: &DeriveInput) -> Result<TokenStream2> {
    let variants = variants(input, "ResponseVariants")?;
    let name = &input.ident;

    // `FieldTy: IntoResponse` for every variant. Spanned on the field type so
    // a missing impl points at the right place rather than at the derive.
    let mut generics = input.generics.clone();
    let where_clause = generics.make_where_clause();
    for v in &variants {
        let ty = v.ty;
        match v.serialize_type {
            Some(SerializeType::Json) => {
                where_clause.predicates.push(parse_quote_spanned! {
                    ty.span() => ::axum::Json<#ty>: ::axum::response::IntoResponse
                })
            },
            None => {
                where_clause
                    .predicates
                    .push(parse_quote_spanned!(ty.span()=> #ty: ::axum::response::IntoResponse));
            }
        }
    }
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let arms = variants.iter().map(|v| {
        let pat = v.pattern();
        let body = match v.serialize_type {
            Some(SerializeType::Json) => {
                quote! {::axum::Json(inner) }
            },
            None => quote! { inner }
        };
        quote!(#pat => ::axum::response::IntoResponse::into_response(#body))
    });

    let froms = expand_from_variants(input)?;

    Ok(quote! {
        impl #impl_generics ::axum::response::IntoResponse for #name #ty_generics #where_clause {
            fn into_response(self) -> ::axum::response::Response {
                match self {
                    #(#arms,)*
                }
            }
        }
        impl #impl_generics ::orae_http_server::responses::ResponseVariants for #name #ty_generics #where_clause {}

        #froms
    })
}


pub fn expand_from_variants(input: &DeriveInput) -> Result<TokenStream2> {
    let variants = variants(input, "FromVariants")?;
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let impls = variants.iter().map(|v| {
        let ty = v.ty;
        let ctor = v.constructor();
        quote! {
            impl #impl_generics ::core::convert::From<#ty> for #name #ty_generics #where_clause {
                fn from(inner: #ty) -> Self {
                    #ctor
                }
            }
        }
    });

    Ok(quote!(#(#impls)*))
}