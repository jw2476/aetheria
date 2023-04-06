use std::{rc::Weak, sync::Mutex};

use proc_macro::TokenStream;
use quote::quote;

#[proc_macro]
pub fn impl_load(input: TokenStream) -> TokenStream {
    let generated = format!("
        impl {0} {{
            pub fn load() -> std::rc::Rc<Self> {{
                static mut HANDLE: std::sync::Mutex<Option<std::rc::Weak<{0}>>> = std::sync::Mutex::new(None);

                unsafe {{
                    let mut handle = HANDLE.lock().unwrap();

                    match handle.as_ref().map(|handle| handle.upgrade()).flatten() {{
                        Some(handle) => handle,
                        None => std::rc::Rc::new_cyclic(|weak| {{
                            *handle = Some(weak.clone());
                            {0}::new()
                        }}),
                    }}
                }}
            }}
        }}
    ", input.to_string());

    generated.parse().unwrap()
}

struct input {}
