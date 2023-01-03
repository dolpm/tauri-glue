extern crate proc_macro;
use proc_macro2::*;
use quote::{format_ident, quote, ToTokens, TokenStreamExt};

fn find_arg(args: &Vec<&[proc_macro::TokenTree]>, arg_name: &str) -> Option<String> {
    match args.iter().find(|&&p| {
        p.len() == 3
            && match &p[0] {
                proc_macro::TokenTree::Ident(v1) => {
                    v1.to_string() == arg_name
                        && match &p[1] {
                            proc_macro::TokenTree::Punct(v2) => v2.as_char() == '=',
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
    t: Vec<TokenTree>,
}

impl ToTokens for BoundFunctionParam {
    fn to_tokens(&self, tokens: &mut quote::__private::TokenStream) {
        tokens.append(format_ident!("{}", self.name));
        tokens.append(Punct::new(':', quote::__private::Spacing::Alone));
        tokens.append_all(self.t.iter());
    }
}

struct BoundFunction {
    name: String,
    is_async: bool,
    is_pub: bool,
    params: Vec<BoundFunctionParam>,
    return_type: Vec<TokenTree>,
}

fn parse_fn(input: TokenStream) -> BoundFunction {
    let input = input.into_iter().collect::<Vec<_>>();
    println!("{:#?}", input);

    let is_async = input
        .iter()
        .find(|p| match p {
            TokenTree::Ident(v) => v.to_string() == "async",
            _ => false,
        })
        .is_some();

    let is_pub = input
        .iter()
        .find(|p| match p {
            TokenTree::Ident(v) => v.to_string() == "pub",
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
            let ts = ts
                .split(|t| match t {
                    TokenTree::Punct(c) => c.as_char() == ',',
                    _ => false,
                })
                .collect::<Vec<&[TokenTree]>>();

            ts.into_iter()
                .map(|tree| BoundFunctionParam {
                    name: tree[0].to_string(),
                    t: tree[2..].to_vec(),
                })
                .collect()
        }
    };

    let mut return_type = vec![TokenTree::Punct(Punct::new(';', Spacing::Alone))];
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
            return_type = input[i..].to_vec();
            break;
        }
    }

    BoundFunction {
        name: fn_name,
        is_async,
        is_pub,
        params,
        return_type,
    }
}

#[proc_macro_attribute]
pub fn bind_command(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args = args.into_iter().collect::<Vec<_>>();
    let args = args
        .split(|t| match t {
            proc_macro::TokenTree::Punct(p) => p.as_char() == ',',
            _ => false,
        })
        .collect::<Vec<_>>();

    let command = match find_arg(&args, "name") {
        Some(c) => c,
        None => panic!("must include the argument: name"),
    };

    let parsed_fn = parse_fn(input.into());

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

        let if_pub = if parsed_fn.is_pub {
            quote! {pub}
        } else {
            quote! {}
        };

        let params = parsed_fn.params;

        let ret = {
            let mut s = TokenStream::new();
            s.append_all(parsed_fn.return_type.iter());
            s
        };

        quote!(
            #[wasm_bindgen(inline_js = #export)]
            extern "C" {
                #catch
                #if_pub #if_async fn #fn_name(#(#params),*) #ret
            }
        )
    };

    bindgen_tokens.into()
}
