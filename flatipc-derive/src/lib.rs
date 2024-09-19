use std::sync::atomic::AtomicUsize;

use proc_macro::TokenStream;
// use quote::quote;
use quote::{format_ident, quote};
use syn::{parse_macro_input, spanned::Spanned, DeriveInput, Ident};
// /// Defines a derive function named `$outer` which parses its input
// /// `TokenStream` as a `DeriveInput` and then invokes the `$inner` function.
// ///
// /// Note that the separate `$outer` parameter is required - proc macro functions
// /// are currently required to live at the crate root, and so the caller must
// /// specify the name in order to avoid name collisions.
// macro_rules! derive {
//     ($trait:ident => $outer:ident => $inner:ident) => {
//         #[proc_macro_derive($trait)]
//         pub fn $outer(ts: proc_macro::TokenStream) -> proc_macro::TokenStream {
//             let ast = syn::parse_macro_input!(ts as DeriveInput);
//             $inner(&ast).into()
//         }
//     };
// }

// derive!(XousIpc => derive_xous_ipc => derive_xous_ipc_inner);

// fn derive_xous_ipc_inner(ast: &DeriveInput) -> proc_macro2::TokenStream {
//     let try_from_bytes = derive_try_from_bytes_inner(ast);
//     let from_zeros = match &ast.data {
//         Data::Struct(strct) => derive_from_zeros_struct(ast, strct),
//         Data::Enum(enm) => derive_from_zeros_enum(ast, enm),
//         Data::Union(unn) => derive_from_zeros_union(ast, unn),
//     };
//     IntoIterator::into_iter([try_from_bytes, from_zeros]).collect()
// }

// fn derive_try_from_bytes_inner(ast: &DeriveInput) -> proc_macro2::TokenStream {
//     match &ast.data {
//         Data::Struct(strct) => derive_try_from_bytes_struct(ast, strct),
//         Data::Enum(enm) => derive_try_from_bytes_enum(ast, enm),
//         Data::Union(unn) => derive_try_from_bytes_union(ast, unn),
//     }
// }

#[proc_macro_derive(XousIpc)]
pub fn derive_xous_ipc(ts: TokenStream) -> TokenStream {
    // println!("AST: {:?}", ts);
    let ast = parse_macro_input!(ts as syn::DeriveInput);
    // Ensure the thing is using a repr we support.
    if let Err(e) = ensure_valid_repr(&ast) {
        return e.into();
    }
    // let try_from_bytes = derive_try_from_bytes_inner(&ast);
    let result = match &ast.data {
        syn::Data::Struct(r#struct) => derive_xous_ipc_struct(&ast, r#struct),
        syn::Data::Enum(r#enum) => derive_xous_ipc_enum(&ast, r#enum),
        syn::Data::Union(r#union) => derive_xous_ipc_union(&ast, r#union),
    };
    eprintln!("TOKENS: {}", result);
    // IntoIterator::into_iter([try_from_bytes, from_zeros]).collect()
    // ts
    // TokenStream::new()
    result
}

fn ensure_valid_repr(ast: &DeriveInput) -> Result<(), TokenStream> {
    let mut repr_c = false;
    let mut valid_align = true;
    for attr in ast.attrs.iter() {
        if attr.path().is_ident("repr") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("C") {
                    repr_c = true;
                    Ok(())
                // } else if meta.path.is_ident("align") {
                //     meta.parse_nested_meta(|v| {
                //         // println!("Align: has value");
                //         // Err(syn::Error::new(ast.span(), "Align must have a value"))
                //         Ok(())
                //     })
                //     .unwrap();
                //     // println!("Path: {:?}", meta.value());
                //     Ok(())
                //     // // let err: TokenStream =
                //     // Err(syn::Error::new(
                //     //     ast.span(),
                //     //     "XousIpc only supports repr(C) structs",
                //     // ))
                //     // // .into();
                //     // // err
                } else {
                    Ok(())
                }
            })
            .map_err(|e| e.to_compile_error())?;
        }
    }
    if !repr_c {
        Err(
            syn::Error::new(ast.span(), "XousIpc only supports repr(C) structs")
                .to_compile_error()
                .into(),
        )
    // } else if !valid_align {
    //     Err(syn::Error::new(ast.span(), "Align is not valid")
    //         .to_compile_error()
    //         .into())
    } else {
        Ok(())
    }
}

fn type_to_string(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Array(_type_array) => "Array".to_owned(),
        syn::Type::BareFn(_type_bare_fn) => "BareFn".to_owned(),
        syn::Type::Group(_type_group) => "Group".to_owned(),
        syn::Type::ImplTrait(_type_impl_trait) => "ImplTrait".to_owned(),
        syn::Type::Infer(_type_infer) => "Infer".to_owned(),
        syn::Type::Macro(_type_macro) => "Macro".to_owned(),
        syn::Type::Never(_type_never) => "Never".to_owned(),
        syn::Type::Paren(_type_paren) => "Paren".to_owned(),
        syn::Type::Path(_type_path) => "Path".to_owned(),
        syn::Type::Ptr(_type_ptr) => "Ptr".to_owned(),
        syn::Type::Reference(_type_reference) => "Reference".to_owned(),
        syn::Type::Slice(_type_slice) => "Slice".to_owned(),
        syn::Type::TraitObject(_type_trait_object) => "TraitObject".to_owned(),
        syn::Type::Tuple(_type_tuple) => "Tuple".to_owned(),
        syn::Type::Verbatim(_token_stream) => "Verbatim".to_owned(),
        _ => "Other (Unknown)".to_owned(),
    }
}

fn ensure_type_exists_for(
    ty: &syn::Type,
) -> Result<proc_macro2::TokenStream, proc_macro2::TokenStream> {
    eprintln!("Type: {}", type_to_string(ty));

    match ty {
        syn::Type::Path(_) => {
            static ATOMIC_INDEX: AtomicUsize = AtomicUsize::new(0);
            let fn_name = format_ident!(
                "assert_type_exists_for_parameter_{}",
                ATOMIC_INDEX.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            );
            Ok(quote! {
                fn #fn_name (_var: #ty) { ensure_is_transmittable::<#ty>(); }
            })
        }
        syn::Type::Tuple(tuple) => {
            let mut check_functions = vec![];
            for ty in tuple.elems.iter() {
                check_functions.push(ensure_type_exists_for(ty)?);
            }
            Ok(quote! {
                #(#check_functions)*
            })
        }
        syn::Type::Array(array) => ensure_type_exists_for(&array.elem),
        _ => Err(syn::Error::new(
            ty.span(),
            format!("This type {} is unexpected", type_to_string(ty)),
        )
        .to_compile_error()
        .into()),
    }
}

fn derive_xous_ipc_enum(ast: &syn::DeriveInput, enm: &syn::DataEnum) -> TokenStream {
    let mut variants = Vec::new();
    let surrounding_function = format_ident!("ensure_members_are_transmittable_for_{}", ast.ident);
    for variant in &enm.variants {
        let fields = match &variant.fields {
            syn::Fields::Named(fields) => fields
                .named
                .iter()
                .map(|f| ensure_type_exists_for(&f.ty))
                .collect(),
            syn::Fields::Unnamed(fields) => fields
                .unnamed
                .iter()
                .map(|f| ensure_type_exists_for(&f.ty))
                .collect(),
            syn::Fields::Unit => Vec::new(),
        };

        let mut vetted_fields = vec![];
        for field in fields {
            match field {
                Ok(f) => vetted_fields.push(f),
                Err(e) => return e.into(),
            }
        }

        variants.push(quote! {
                #(#vetted_fields)*
        });
    }
    quote! {
        fn #surrounding_function () {
            pub fn ensure_is_transmittable<T: crate::Transmittable>() {}
            #(#variants)*
        }
    }
    .into()
}

fn derive_xous_ipc_struct(ast: &syn::DeriveInput, strct: &syn::DataStruct) -> TokenStream {
    let surrounding_function = format_ident!("ensure_members_are_transmittable_for_{}", ast.ident);
    let fields = match &strct.fields {
        syn::Fields::Named(fields) => fields
            .named
            .iter()
            .map(|f| ensure_type_exists_for(&f.ty))
            .collect(),
        syn::Fields::Unnamed(fields) => fields
            .unnamed
            .iter()
            .map(|f| ensure_type_exists_for(&f.ty))
            .collect(),
        syn::Fields::Unit => Vec::new(),
    };
    let mut vetted_fields = vec![];
    for field in fields {
        match field {
            Ok(f) => vetted_fields.push(f),
            Err(e) => return e.into(),
        }
    }
    quote! {
        pub fn ensure_is_transmittable<T: crate::Transmittable>() {}
        fn #surrounding_function () {
            #(#vetted_fields)*
        }
    }
    .into()
}

fn derive_xous_ipc_union(ast: &syn::DeriveInput, unn: &syn::DataUnion) -> TokenStream {
    // let ident = &ast.ident;
    // let fields = unn.fields.named.iter().map(|f| {
    //     let ident = &f.ident;
    //     let ty = &f.ty;
    //     quote! {
    //         #ident: Default::default(),
    //     }
    // }).collect();
    // quote! {
    //     impl Default for #ident {
    //         fn default() -> Self {
    //             Self {
    //                 #(#fields)*
    //             }
    //         }
    //     }
    // }
    TokenStream::new()
}
