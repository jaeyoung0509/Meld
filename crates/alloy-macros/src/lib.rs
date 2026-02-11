use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::Parse, parse_macro_input, parse_quote, Error, FnArg, GenericArgument, Ident, ItemFn,
    LitStr, Pat, PatTupleStruct, PathArguments, Token, Type,
};

struct RouteArgs {
    method: RouteMethod,
    path: LitStr,
    auto_validate: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum RouteMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl Parse for RouteArgs {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let method_ident: Ident = input.parse()?;
        let method = match method_ident.to_string().as_str() {
            "get" => RouteMethod::Get,
            "post" => RouteMethod::Post,
            "put" => RouteMethod::Put,
            "patch" => RouteMethod::Patch,
            "delete" => RouteMethod::Delete,
            _ => {
                return Err(Error::new(
                    method_ident.span(),
                    "unsupported method; use one of: get, post, put, patch, delete",
                ))
            }
        };
        if input.is_empty() {
            return Err(Error::new(method_ident.span(), "route path is required"));
        }
        input.parse::<Token![,]>()?;

        let path: LitStr = input
            .parse()
            .map_err(|_| Error::new(input.span(), "route path must be a string literal"))?;

        let mut auto_validate = false;
        while !input.is_empty() {
            input.parse::<Token![,]>()?;
            let flag: Ident = input.parse()?;
            match flag.to_string().as_str() {
                "auto_validate" => auto_validate = true,
                _ => return Err(Error::new(flag.span(), format!("unknown flag `{}`", flag))),
            }
        }

        Ok(Self {
            method,
            path,
            auto_validate,
        })
    }
}

#[proc_macro_attribute]
pub fn route(args: TokenStream, item: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(args as RouteArgs);
    let mut item_fn = parse_macro_input!(item as ItemFn);

    if parsed.auto_validate {
        apply_auto_validate(&mut item_fn);
    }

    let _ = (parsed.method, parsed.path);

    TokenStream::from(quote! { #item_fn })
}

fn apply_auto_validate(item_fn: &mut ItemFn) {
    for input in &mut item_fn.sig.inputs {
        if let FnArg::Typed(arg) = input {
            maybe_rewrite_typed_arg(arg);
        }
    }
}

fn maybe_rewrite_typed_arg(arg: &mut syn::PatType) {
    let Some(rewritten_path) = rewrite_extractor_type(&mut arg.ty) else {
        return;
    };

    if let Pat::TupleStruct(PatTupleStruct { path, .. }) = arg.pat.as_mut() {
        *path = rewritten_path;
    }
}

fn rewrite_extractor_type(ty: &mut Box<Type>) -> Option<syn::Path> {
    let Type::Path(type_path) = ty.as_mut() else {
        return None;
    };

    let segment = type_path.path.segments.last()?;
    let inner_ty = extract_single_generic_type(&segment.arguments)?;
    let rewritten_path: syn::Path = match segment.ident.to_string().as_str() {
        "Json" => parse_quote!(::alloy_server::api::ValidatedJson),
        "Query" => parse_quote!(::alloy_server::api::ValidatedQuery),
        "Path" => parse_quote!(::alloy_server::api::ValidatedPath),
        _ => return None,
    };
    let rewritten_ty: Type = parse_quote!(#rewritten_path<#inner_ty>);

    if !matches!(rewritten_ty, Type::Path(_)) {
        return None;
    }
    *ty = Box::new(rewritten_ty);
    Some(rewritten_path)
}

fn extract_single_generic_type(arguments: &PathArguments) -> Option<Type> {
    let PathArguments::AngleBracketed(args) = arguments else {
        return None;
    };
    if args.args.len() != 1 {
        return None;
    }
    match args.args.first() {
        Some(GenericArgument::Type(ty)) => Some(ty.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::{parse_quote, parse_str};

    #[test]
    fn parses_method_path_and_auto_validate_flag() {
        let parsed = parse_str::<RouteArgs>(r#"post, "/notes", auto_validate"#)
            .expect("route args should parse");

        assert_eq!(parsed.method, RouteMethod::Post);
        assert_eq!(parsed.path.value(), "/notes");
        assert!(parsed.auto_validate);
    }

    #[test]
    fn parses_without_auto_validate() {
        let parsed = parse_str::<RouteArgs>(r#"get, "/health""#).expect("route args should parse");

        assert_eq!(parsed.method, RouteMethod::Get);
        assert_eq!(parsed.path.value(), "/health");
        assert!(!parsed.auto_validate);
    }

    #[test]
    fn rejects_unsupported_method() {
        let err = match parse_str::<RouteArgs>(r#"options, "/notes""#) {
            Ok(_) => panic!("unsupported method must fail"),
            Err(err) => err,
        };
        assert!(err.to_string().contains("unsupported method"));
    }

    #[test]
    fn rejects_unknown_flag() {
        let err = match parse_str::<RouteArgs>(r#"post, "/notes", unknown_flag"#) {
            Ok(_) => panic!("unknown flag must fail"),
            Err(err) => err,
        };

        let message = err.to_string();
        assert!(message.contains("unknown flag"));
    }

    #[test]
    fn rejects_missing_path() {
        let err = match parse_str::<RouteArgs>("post") {
            Ok(_) => panic!("missing path must fail"),
            Err(err) => err,
        };
        assert!(err.to_string().contains("path"));
    }

    #[test]
    fn rejects_non_string_path() {
        let err = match parse_str::<RouteArgs>("post, 10") {
            Ok(_) => panic!("non-string path must fail"),
            Err(err) => err,
        };
        assert!(err.to_string().contains("string"));
    }

    #[test]
    fn auto_validate_rewrites_json_query_and_path_extractors() {
        let mut item_fn: ItemFn = parse_quote! {
            async fn create_note(
                Query(q): Query<ListQuery>,
                Json(body): Json<CreateNote>,
                Path(path): Path<NotePath>,
            ) {}
        };

        apply_auto_validate(&mut item_fn);

        let first = item_fn
            .sig
            .inputs
            .iter()
            .next()
            .expect("first arg should exist");
        let second = item_fn
            .sig
            .inputs
            .iter()
            .nth(1)
            .expect("second arg should exist");
        let third = item_fn
            .sig
            .inputs
            .iter()
            .nth(2)
            .expect("third arg should exist");

        assert_eq!(arg_type_ident(first), Some("ValidatedQuery".to_string()));
        assert_eq!(arg_pat_ident(first), Some("ValidatedQuery".to_string()));
        assert_eq!(arg_type_ident(second), Some("ValidatedJson".to_string()));
        assert_eq!(arg_pat_ident(second), Some("ValidatedJson".to_string()));
        assert_eq!(arg_type_ident(third), Some("ValidatedPath".to_string()));
        assert_eq!(arg_pat_ident(third), Some("ValidatedPath".to_string()));
    }

    #[test]
    fn without_auto_validate_keeps_original_extractors() {
        let mut item_fn: ItemFn = parse_quote! {
            async fn create_note(Query(q): Query<ListQuery>, Json(body): Json<CreateNote>) {}
        };
        let args = parse_str::<RouteArgs>(r#"post, "/notes""#).expect("route args should parse");

        if args.auto_validate {
            apply_auto_validate(&mut item_fn);
        }

        let rendered = quote!(#item_fn).to_string();
        assert!(rendered.contains("Query"));
        assert!(rendered.contains("Json"));
        assert!(!rendered.contains("ValidatedQuery"));
        assert!(!rendered.contains("ValidatedJson"));
    }

    fn arg_type_ident(arg: &FnArg) -> Option<String> {
        let FnArg::Typed(arg) = arg else {
            return None;
        };
        let Type::Path(type_path) = arg.ty.as_ref() else {
            return None;
        };
        type_path.path.segments.last().map(|s| s.ident.to_string())
    }

    fn arg_pat_ident(arg: &FnArg) -> Option<String> {
        let FnArg::Typed(arg) = arg else {
            return None;
        };
        let Pat::TupleStruct(tuple_struct) = arg.pat.as_ref() else {
            return None;
        };
        tuple_struct
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
    }
}
