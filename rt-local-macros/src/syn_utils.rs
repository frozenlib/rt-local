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
