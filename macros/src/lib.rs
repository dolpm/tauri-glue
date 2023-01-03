extern crate proc_macro;
use proc_macro::*;
use quote::{format_ident, quote, ToTokens, TokenStreamExt};

fn find_arg(args: &Vec<&[TokenTree]>, arg_name: &str) -> Option<String> {
    match args.iter().find(|&&p| {
        p.len() == 3
            && match &p[0] {
                TokenTree::Ident(v1) => {
                    v1.to_string() == arg_name
                        && match &p[1] {
                            TokenTree::Punct(v2) => v2.as_char() == '=',
                            _ => false,
                        }
                }
                _ => false,
            }
    }) {
        Some(&c) => Some(c[2].to_string()),
        None => None,
    }
}

struct BoundFunctionParam {
    name: String,
    t: String,
}

impl ToTokens for BoundFunctionParam {
    fn to_tokens(&self, tokens: &mut quote::__private::TokenStream) {
        tokens.append(format_ident!("{}", self.name));
        tokens.append(quote::__private::Punct::new(
            ':',
            quote::__private::Spacing::Alone,
        ));
        tokens.append(format_ident!("{}", self.t));
    }
}

struct BoundFunction {
    name: String,
    is_async: bool,
    params: Vec<BoundFunctionParam>,
    return_type: String,
}

fn parse_fn(input: TokenStream) -> BoundFunction {
    let input = input.into_iter().collect::<Vec<_>>();

    let is_async = input
        .iter()
        .find(|p| match p {
            TokenTree::Ident(v) => v.to_string() == "async",
            _ => false,
        })
        .is_some();

    let fn_name = match &input[input
        .iter()
        .enumerate()
        .find_map(|(i, p)| match p {
            TokenTree::Ident(v) => {
                if v.to_string() == "fn" {
                    Some(i + 1)
                } else {
                    None
                }
            }
            _ => None,
        })
        .expect("couldn't find function name")]
    {
        TokenTree::Ident(v) => v.to_string(),
        _ => panic!("couldn't find function name"),
    };

    let params = match input.iter().find_map(|p| match p {
        TokenTree::Group(g) => Some(g),
        _ => None,
    }) {
        None => vec![],
        Some(g) => {
            let ts = g.stream().into_iter().collect::<Vec<_>>();
            let mut result = vec![];
            for (i, t) in ts.iter().enumerate() {
                if i == 0
                    || (i + 1 < ts.len()
                        && match &ts[i + 1] {
                            TokenTree::Punct(p) => p.as_char() == ':',
                            _ => false,
                        })
                {
                    result.push(BoundFunctionParam {
                        name: t.to_string(),
                        t: ts[i + 2].to_string(),
                    });
                }
            }
            result
        }
    };

    let mut return_type = ";".to_string();
    for (i, t) in input.iter().enumerate() {
        if match t {
            TokenTree::Punct(v1) => {
                v1.as_char() == '-'
                    && match &input[i + 1] {
                        TokenTree::Punct(v2) => v2.as_char() == '>',
                        _ => false,
                    }
            }
            _ => false,
        } {
            return_type = input[i..]
                .to_vec()
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<String>>()
                .join("");
        }
    }

    BoundFunction {
        name: fn_name,
        is_async,
        params,
        return_type,
    }
}

#[proc_macro_attribute]
pub fn bind_command(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = args.into_iter().collect::<Vec<_>>();
    let args = args
        .split(|t| match t {
            TokenTree::Punct(p) => p.as_char() == ',',
            _ => false,
        })
        .collect::<Vec<_>>();

    let command = match find_arg(&args, "name") {
        Some(c) => c,
        None => panic!("must include the argument: name"),
    };

    let parsed_fn = parse_fn(input);

    let bindgen_tokens = {
        let fn_param_names = parsed_fn
            .params
            .iter()
            .map(|p| p.name.clone())
            .collect::<Vec<_>>()
            .join(",");

        let export = format!(
            "export async function {command}({fn_param_names}) {{ return await window.__TAURI_INVOKE__('{command}', {{ {fn_param_names} }}); }}",
        );

        let fn_name = format_ident!("{}", parsed_fn.name);

        let catch = quote! {
            #[wasm_bindgen(catch)]
        };

        let if_async = if parsed_fn.is_async {
            quote! {async}
        } else {
            quote! {}
        };

        // TODO: pub

        let params = parsed_fn.params;

        let ret = parsed_fn
            .return_type
            .parse::<quote::__private::TokenStream>()
            .expect("failed");

        quote!(
            #[wasm_bindgen(inline_js = #export)]
            extern "C" {
                #catch
                pub #if_async fn #fn_name(#(#params),*) #ret
            }
        )

    };

    bindgen_tokens.into()
}
