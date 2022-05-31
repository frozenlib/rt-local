use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{parse2, spanned::Spanned, ItemFn, Result};

pub fn build(
    _attr: TokenStream,
    item: TokenStream,
    backend: &str,
    is_test: bool,
) -> Result<TokenStream> {
    if let Ok(mut item_fn) = parse2::<ItemFn>(item) {
        if item_fn.sig.asyncness.is_none() {
            bail!(
                item_fn.sig.span(),
                "the `async` keyword is missing from the function declaration"
            );
        }
        item_fn.sig.asyncness = None;
        let attrs = &item_fn.attrs;
        let vis = &item_fn.vis;
        let sig = &item_fn.sig;
        let stmts = &item_fn.block.stmts;
        let test = if is_test {
            quote!(#[::core::prelude::v1::test])
        } else {
            quote!()
        };
        let backend = format_ident!("{}", backend);
        Ok(quote! {
            #test
            #(#attrs)*
            #vis #sig {
                ::rt_local::runtime::#backend::run(async {
                    #(#stmts)*
                })
            }
        })
    } else {
        let name = if is_test { "test" } else { "main" };
        bail!(
            Span::call_site(),
            "`#[{}]` can apply to only function",
            name
        );
    }
}
