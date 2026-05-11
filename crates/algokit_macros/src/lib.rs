use proc_macro::TokenStream;
use quote::quote;
use syn::{FnArg, ItemFn, PatType, Type, parse_macro_input};

fn is_active_avm_type(ty: &Type) -> bool {
    match ty {
        Type::Reference(tr) => {
            if let Type::Path(tp) = &*tr.elem {
                tp.path
                    .segments
                    .last()
                    .map(|s| s.ident == "ActiveAvm")
                    .unwrap_or(false)
            } else {
                false
            }
        }
        Type::Path(tp) => tp
            .path
            .segments
            .last()
            .map(|s| s.ident == "ActiveAvm")
            .unwrap_or(false),
        _ => false,
    }
}

/// Wraps a function that takes exactly one `ActiveAvm` parameter and generates:
/// 1. A global allocator (when not in test mode)
/// 2. A `program` entry point function that calls the wrapped function
///
/// Example:
/// ```ignore
/// #[program_entry]
/// fn state_loop(avm: ActiveAvm) -> u64 {
///     // ...
/// }
/// ```
///
/// Generates:
/// ```ignore
/// #[cfg(not(test))]
/// #[global_allocator]
/// static GLOBAL_ALLOCATOR: algokit::AvmHostAllocator = algokit::AvmHostAllocator;
///
/// #[unsafe(export_name = "program")]
/// pub extern "C" fn program() -> u64 {
///     state_loop(host_avm())
/// }
/// ```
#[proc_macro_attribute]
pub fn program_entry(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);

    let fn_name = &input_fn.sig.ident;
    let fn_vis = &input_fn.vis;
    let fn_sig = &input_fn.sig;
    let fn_block = &input_fn.block;
    let fn_attrs = &input_fn.attrs;
    let return_type = &fn_sig.output;

    // Check function has exactly one parameter
    if input_fn.sig.inputs.len() != 1 {
        return syn::Error::new_spanned(
            &input_fn.sig.inputs,
            "#[program_entry] function must take exactly one `ActiveAvm` parameter",
        )
        .to_compile_error()
        .into();
    }

    // Verify first parameter is ActiveAvm
    let first_param = fn_sig.inputs.first();
    let is_active_avm = matches!(
        first_param,
        Some(FnArg::Typed(PatType { ty, .. })) if is_active_avm_type(ty)
    );

    if !is_active_avm {
        return syn::Error::new_spanned(
            input_fn.sig.fn_token,
            "#[program_entry] function must have `ActiveAvm` as its parameter type",
        )
        .to_compile_error()
        .into();
    }

    let expanded = quote! {
        #(#fn_attrs)*
        #fn_vis #fn_sig #fn_block

        #[cfg(not(test))]
        #[global_allocator]
        static GLOBAL_ALLOCATOR: ::algokit::AvmHostAllocator = ::algokit::AvmHostAllocator;

        extern crate alloc;

        #[unsafe(export_name = "program")]
        pub extern "C" fn program() #return_type {
            #fn_name(alloc::boxed::Box::leak(alloc::boxed::Box::new(::algokit::HostAvm {})))
        }
    };

    TokenStream::from(expanded)
}
