#![crate_name = "my_proc_macro"]
#![crate_type = "proc-macro"]

use std::iter::FromIterator;

extern crate proc_macro;
use proc_macro::TokenStream;

#[proc_macro]
pub fn reg8(item: TokenStream) -> TokenStream {
    TokenStream::from_iter(item.into_iter().map(|t| {
        format!(
            "const fn {}(&self) -> &Reg<u8> {{ self.r8(RegId8::{}) }}",
            t.to_string(),
            t.to_string().to_uppercase()
        )
        .parse::<TokenStream>()
        .unwrap()
    }))
}

#[proc_macro]
pub fn reg16(item: TokenStream) -> TokenStream {
    TokenStream::from_iter(item.into_iter().map(|t| {
        format!(
            "const fn {}(&self) -> &Reg<u16> {{ self.r16(RegId16::{}) }}",
            t.to_string(),
            t.to_string().to_uppercase()
        )
        .parse::<TokenStream>()
        .unwrap()
    }))
}
