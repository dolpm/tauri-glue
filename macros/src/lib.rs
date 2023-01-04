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
    pred: Vec<TokenTree>,
    fn_name: TokenTree,
    params: Vec<BoundFunctionParam>,
    rest: Vec<TokenTree>,
}

fn parse_fn(input: TokenStream) -> BoundFunction {
    let input = input.into_iter().collect::<Vec<_>>();

    let pred = input
        .iter()
        .enumerate()
        .find_map(|(i, t)| match t {
            TokenTree::Ident(v) => {
                if v.to_string() == "fn" {
                    Some(i + 1)
                } else {
                    None
                }
            }
            _ => None,
        })
        .expect("failed to find pred");

    let params = match &input[pred + 1] {
        TokenTree::Group(g) => {
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
        _ => panic!("couldn't find group"),
    };

    BoundFunction {
        pred: input[..pred].to_vec(),
        fn_name: input[pred].clone(),
        params,
        rest: input[pred + 2..input.len()].to_vec(),
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
            "export async function __{command}({fn_param_names}) {{ return await window.__TAURI_INVOKE__('{command}', {{ {fn_param_names} }}); }}",
        );

        let (pred, priv_fn_name, pub_fn_name, params, rest) = (
            {
                let mut stream = TokenStream::new();
                stream.append_all(parsed_fn.pred);
                stream
            },
            format_ident!("__{}", parsed_fn.fn_name.to_string()),
            parsed_fn.fn_name,
            parsed_fn.params,
            {
                let mut stream = TokenStream::new();
                stream.append_all(parsed_fn.rest[..parsed_fn.rest.len() - 1].iter());
                stream
            },
        );

        let param_names = params
            .iter()
            .map(|p| format_ident!("{}", p.name))
            .collect::<Vec<_>>();

        let await_priv = if pred.clone().into_iter().any(|v| v.to_string() == "async") {
            let mut stream = TokenStream::new();
            stream.append(TokenTree::Punct(Punct::new('.', Spacing::Alone)));
            stream.append(format_ident!("await"));
            stream
        } else {
            TokenStream::new()
        };

        quote!(
            #[_wasm_bindgen(inline_js = #export)]
            extern "C" {
                #pred #priv_fn_name(#(#param_names: String),*) -> _jsv;
            }

            #pred #pub_fn_name(#(#params),*) #rest {
                _bincode_deserialize(
                    &_u8a::new(
                        &#priv_fn_name(
                            #(
                                _bincode_serialize(&#param_names)
                                    .expect("failed to serialize parameter")
                                    .to_json()
                                    .unwrap()
                            ),*
                        )#await_priv,
                    )
                    .to_vec()[..],
                )
                .expect("failed to deserialize payload")
            }
        )
    };

    bindgen_tokens.into()
}

#[proc_macro_attribute]
pub fn command(
    _args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let parsed_fn = parse_fn(input.into());

    let (pred, priv_fn_name, pub_fn_name, params, rest) = (
        {
            let mut stream = TokenStream::new();
            stream.append_all(parsed_fn.pred);
            stream
        },
        format_ident!("__{}", parsed_fn.fn_name.to_string()),
        parsed_fn.fn_name,
        parsed_fn.params,
        {
            let mut stream = TokenStream::new();
            stream.append_all(parsed_fn.rest);
            stream
        },
    );

    let param_names = params
        .iter()
        .map(|p| format_ident!("{}", p.name))
        .collect::<Vec<_>>();

    let tokens = {
        quote!(
            #pred #priv_fn_name(#(#params),*) #rest

            #[tauri::command]
            fn #pub_fn_name(#(#param_names: &str),*) -> Vec<u8> {
                _bincode_serialize(&__hello(
                    #(
                        _bincode_deserialize(
                            &_serde_json::from_str::<Vec<u8>>(#param_names).unwrap()[..]
                        ).expect("failed to deserialize parameter")
                    ),*
                ))
                .expect("failed to serialize")
            }
        )
    };

    tokens.into()
}
