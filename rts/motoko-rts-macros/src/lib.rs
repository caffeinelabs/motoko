use proc_macro::TokenStream;
use quote::quote;

/// This macro is used to generate monomorphic versions of allocating RTS functions, to allow
/// calling such functions in generated code. Example:
///
/// ```
/// #[ic_mem_fn]
/// pub unsafe fn text_concat<M: Memory>(mem: &mut M, s1: SkewedPtr, s2: SkewedPtr) -> SkewedPtr {
///     ...
/// }
/// ```
///
/// This functions has a `Memory` parameter to be able to allocate on heap. To compile this
/// function to use in generated code we need a monomorphic version, without a `Memory` parameter.
/// This macro generates the monomorphic version. Macro expansion looks like this:
///
/// ```
/// // Original function generated directly, to allow use from the test suite
/// pub unsafe fn text_concat<M: Memory>(mem: &mut M, s1: SkewedPtr, s2: SkewedPtr) -> SkewedPtr {
///     ...
/// }
///
/// // New, monomorphic version
/// #[cfg(feature = "ic")]
/// #[export_name = "text_concat"]
/// unsafe extern "C" fn ic_text_concat(s1: SkewedPtr, s2: SkewedPtr) -> SkewedPtr {
///     text_concat(crate::memory::ic::IcMemory, s1, s2)
/// }
/// ```
///
/// Reminder: `ic` feature is used when compiling the RTS to be linked with generated code. It's
/// disabled when compiling for testing.
///
/// `ic_mem_fn` takes an optional `ic_only` attribute which adds a `cfg(feature = "ic")` guard to
/// the original function:
///
/// ```
/// #[ic_mem_fn(ic_only)]
/// fn my_function<M: Memory>(mem: &mut M) { ... }
/// ```
///
/// Expansion:
///
/// ```
/// #[cfg(feature = "ic")]
/// fn my_function<M: Memory>(mem: &mut M) { ... }
///
/// #[cfg(feature = "ic")]
/// #[export_name = "text_concat"]
/// unsafe extern "C" fn ic_my_function() {
///     my_function(crate::memory::ic::IcMemory)
/// }
/// ```
///
/// This is useful when the function won't be used when compiling the RTS for testing.
#[proc_macro_attribute]
pub fn ic_mem_fn(attr: TokenStream, input: TokenStream) -> TokenStream {
    let ic_only = if attr.is_empty() {
        false
    } else if attr.to_string() == "ic_only" {
        true
    } else {
        panic!("Unknown attribute: {:?}", attr.to_string());
    };

    let fun = syn::parse_macro_input!(input as syn::ItemFn);
    let fun_sig = &fun.sig;

    // Some sanity checks
    assert!(fun_sig.asyncness.is_none(), "IC functions cannot be async");
    assert_eq!(
        fun_sig.generics.params.len(),
        1,
        "IC memory functions should have one generic argument for the memory implementation"
    );
    assert!(
        fun_sig.abi.is_none(),
        "Functions with #[ic_fn] attribute cannot have ABI annotations"
    );
    assert!(
        fun_sig.variadic.is_none(),
        "IC functions cannot have variadic arguments"
    );

    let fn_ident = &fun_sig.ident;
    let fn_wrapper_ident = syn::Ident::new(&format!("ic_{}", fn_ident), fn_ident.span());
    let fn_name = fn_ident.to_string();
    let wrapper_ret = fun_sig.output.clone();
    let wrapper_args: Vec<(syn::Ident, syn::Type)> = fun_sig
        .inputs
        .iter()
        .enumerate()
        .filter_map(|(i, arg)| match arg {
            syn::FnArg::Receiver(_) => {
                panic!("IC functions can't have receivers (`&self`, `&mut self`, etc.)")
            }
            syn::FnArg::Typed(pat) => {
                if i == 0 {
                    // First argument should be `memory`, skip
                    None
                } else {
                    Some((
                        syn::Ident::new(&format!("arg{}", i), proc_macro2::Span::call_site()),
                        (*pat.ty).clone(),
                    ))
                }
            }
        })
        .collect();

    // Parameters of the wrapper function
    let wrapper_params_syn: Vec<proc_macro2::TokenStream> = wrapper_args
        .iter()
        .map(|(ident, ty)| quote!(#ident: #ty))
        .collect();

    // Arguments passed to the original function
    let wrapper_args_syn: Vec<&syn::Ident> = wrapper_args.iter().map(|(ident, _)| ident).collect();

    let fun_attr = if ic_only {
        quote!(#[cfg(feature = "ic")])
    } else {
        quote!()
    };

    quote!(
        #fun_attr
        #fun

        #[cfg(feature = "ic")]
        #[unsafe(export_name = #fn_name)]
        unsafe extern "C" fn #fn_wrapper_ident(#(#wrapper_params_syn,)*) #wrapper_ret {
            #fn_ident(&mut crate::memory::ic::IcMemory, #(#wrapper_args_syn,)*)
        }
    )
    .into()
}

/// Feature macro for incremental GC features, in particular forwarding pointers.
/// Equivalent to using the attribute `#[cfg(feature = "incremental_gc")]`.
#[proc_macro_attribute]
pub fn incremental_gc(attr: TokenStream, input: TokenStream) -> TokenStream {
    assert!(attr.is_empty());
    let block = syn::parse_macro_input!(input as syn::Item);
    quote!(
        #[cfg(feature = "incremental_gc")]
        #block
    )
    .into()
}

/// Feature macro for non-incremental GC features, in particular forwarding pointers.
/// Equivalent to using the attribute `#[cfg(not(feature = "incremental_gc"))]`.
#[proc_macro_attribute]
pub fn non_incremental_gc(attr: TokenStream, input: TokenStream) -> TokenStream {
    assert!(attr.is_empty());
    let block = syn::parse_macro_input!(input as syn::Item);
    quote!(
        #[cfg(not(feature = "incremental_gc"))]
        #block
    )
    .into()
}

#[proc_macro]
pub fn is_incremental_gc(_item: TokenStream) -> TokenStream {
    "cfg!(feature = \"incremental_gc\")".parse().unwrap()
}

/// Feature macro for enhanced orthogonal persistence.
/// Equivalent to using the attribute `#[cfg(feature = "enhanced_orthogonal_persistence")]`.
#[proc_macro_attribute]
pub fn enhanced_orthogonal_persistence(attr: TokenStream, input: TokenStream) -> TokenStream {
    assert!(attr.is_empty());
    let block = syn::parse_macro_input!(input as syn::Item);
    quote!(
        #[cfg(feature = "enhanced_orthogonal_persistence")]
        #block
    )
    .into()
}

/// Feature macro for classical persistence, based on Candid stabilization.
/// Equivalent to using the attribute `#[cfg(not(feature = "enhanced_orthogonal_persistence"))]`.
#[proc_macro_attribute]
pub fn classical_persistence(attr: TokenStream, input: TokenStream) -> TokenStream {
    assert!(attr.is_empty());
    let block = syn::parse_macro_input!(input as syn::Item);
    quote!(
        #[cfg(not(feature = "enhanced_orthogonal_persistence"))]
        #block
    )
    .into()
}

#[proc_macro]
pub fn uses_enhanced_orthogonal_persistence(_item: TokenStream) -> TokenStream {
    "cfg!(feature = \"enhanced_orthogonal_persistence\")"
        .parse()
        .unwrap()
}
