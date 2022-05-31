use proc_macro::TokenStream;
use syn_utils::resolve_attr;

#[macro_use]
mod syn_utils;

mod runtime_main;

use runtime_main::build;

#[proc_macro_attribute]
pub fn core_test(attr: TokenStream, item: TokenStream) -> TokenStream {
    resolve_attr(attr, item, |attr, item| build(attr, item, "core", true))
}

#[proc_macro_attribute]
pub fn core_main(attr: TokenStream, item: TokenStream) -> TokenStream {
    resolve_attr(attr, item, |attr, item| build(attr, item, "core", false))
}

#[proc_macro_attribute]
pub fn windows_test(attr: TokenStream, item: TokenStream) -> TokenStream {
    resolve_attr(attr, item, |attr, item| build(attr, item, "windows", true))
}

#[proc_macro_attribute]
pub fn windows_main(attr: TokenStream, item: TokenStream) -> TokenStream {
    resolve_attr(attr, item, |attr, item| build(attr, item, "windows", false))
}
