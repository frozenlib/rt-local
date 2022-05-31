use proc_macro2::TokenStream;
use syn::Result;

macro_rules! bail {
    (_, $($arg:tt)*) => {
        bail!(proc_macro2::Span::call_site(), $($arg)*)
    };
    ($span:expr, $message:literal $(,)?) => {
        return std::result::Result::Err(syn::Error::new($span, $message))
    };
    ($span:expr, $err:expr $(,)?) => {
        return std::result::Result::Err(syn::Error::new($span, $err))
    };
    ($span:expr, $fmt:expr, $($arg:tt)*) => {
        return std::result::Result::Err(syn::Error::new($span, std::format!($fmt, $($arg)*)))
    };
}

pub fn resolve_attr(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
    build: impl FnOnce(TokenStream, TokenStream) -> Result<TokenStream>,
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
