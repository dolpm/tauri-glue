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
    params: Vec<BoundFunctionParam>,
    return_type: Vec<TokenTree>,
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
        pred: input[..=pred].to_vec(),
        params,
        return_type: input[pred + 2..].to_vec(),
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

        let (pred, params, return_type) = (
            {
                let mut stream = TokenStream::new();
                stream.append_all(parsed_fn.pred);
                stream
            },
            parsed_fn.params,
            {
                let mut stream = TokenStream::new();
                stream.append_all(parsed_fn.return_type);
                stream
            },
        );

        // TODO: remove default catch
        quote!(
            #[wasm_bindgen(inline_js = #export)]
            extern "C" {
                #[wasm_bindgen(catch)]
                #pred(#(#params),*) #return_type
            }
        )
    };

    bindgen_tokens.into()
}
