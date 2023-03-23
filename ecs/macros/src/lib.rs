extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use std::sync::atomic::{AtomicU64, Ordering};
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Component)]
pub fn derive_component(input: TokenStream) -> TokenStream {
    static mut A: AtomicU64 = AtomicU64::new(0);
    static mut B: AtomicU64 = AtomicU64::new(1);

    let mut counter;

    unsafe {
        let a = A.load(Ordering::Acquire) as u128;
        let b = B.load(Ordering::Acquire) as u128;

        counter = (a << 64) | b;

        let DeriveInput { ident, .. } = parse_macro_input!(input);
        let output = quote! {
            impl Component for #ident {
                fn get_id(&self) -> u128 { #counter }
                fn id() -> u128 { #counter }
            }
        };

        counter <<= 1;

        let lower = u64::MAX as u128;
        let upper = !lower;

        let a = ((counter & upper) >> 64) as u64;
        let b = (counter & lower) as u64;

        A.store(a, Ordering::Release);
        B.store(b, Ordering::Release);

        output.into()
    }
}
