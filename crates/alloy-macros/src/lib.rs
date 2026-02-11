use proc_macro::TokenStream;
use quote::quote;
use syn::{parse::Parse, parse_macro_input, Error, Ident, ItemFn, LitStr, Token};

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
                _ => {
                    return Err(Error::new(
                        flag.span(),
                        format!("unknown flag `{}`", flag),
                    ))
                }
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
    let item_fn = parse_macro_input!(item as ItemFn);
    let _ = (parsed.method, parsed.path, parsed.auto_validate);

    TokenStream::from(quote! { #item_fn })
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_str;

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
        let parsed = parse_str::<RouteArgs>(r#"get, "/health""#)
            .expect("route args should parse");

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
}
