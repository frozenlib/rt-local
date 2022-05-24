use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{parse2, spanned::Spanned, ItemFn, Result};

#[macro_use]
mod syn_utils;

#[proc_macro_attribute]
pub fn rt_local_test(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut item: TokenStream = item.into();
    match build(attr.into(), item.clone()) {
        Ok(s) => s,
        Err(e) => {
            item.extend(e.to_compile_error());
            item
        }
    }
    .into()
}

fn build(_attr: TokenStream, item: TokenStream) -> Result<TokenStream> {
    let msg = "`#[rt_local_test]` can only be apply to async functions";
    if let Ok(mut item_fn) = parse2::<ItemFn>(item) {
        if item_fn.sig.asyncness.is_none() {
            bail!(item_fn.sig.span(), "{}", msg);
        }

        item_fn.sig.asyncness = None;
        let attrs = &item_fn.attrs;
        let vis = &item_fn.vis;
        let sig = &item_fn.sig;
        let stmts = &item_fn.block.stmts;
        Ok(quote! {
            #[test]
            #(#attrs)*
            #vis #sig {
                ::rt_local::runtime::core::run(async {
                    #(#stmts)*
                })
            }
        })
    } else {
        bail!(Span::call_site(), "{}", msg);
    }
}
