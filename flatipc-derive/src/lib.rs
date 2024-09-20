use std::sync::atomic::AtomicUsize;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, spanned::Spanned, DeriveInput};

fn ast_hash(ast: &syn::DeriveInput) -> u32 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    ast.hash(&mut hasher);
    let full_hash = hasher.finish();
    ((full_hash >> 32) as u32) ^ (full_hash as u32)
}

#[proc_macro_derive(IpcSafe)]
pub fn derive_transmittable(ts: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(ts as syn::DeriveInput);
    derive_transmittable_inner(ast).unwrap_or_else(|e| e).into()
}

fn derive_transmittable_inner(
    ast: DeriveInput,
) -> Result<proc_macro2::TokenStream, proc_macro2::TokenStream> {
    let ident = ast.ident.clone();
    let transmittable_checks = match &ast.data {
        syn::Data::Struct(r#struct) => generate_transmittable_checks_struct(&ast, r#struct)?,
        syn::Data::Enum(r#enum) => generate_transmittable_checks_enum(&ast, r#enum)?,
        syn::Data::Union(r#union) => generate_transmittable_checks_union(&ast, r#union)?,
    };
    let result = quote! {
        #transmittable_checks

        unsafe impl crate::IpcSafe for #ident {}
    };

    // eprintln!("TRANSMITTABLE: {}", result);
    Ok(result)
}

#[proc_macro_derive(Ipc)]
pub fn derive_ipc(ts: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(ts as syn::DeriveInput);
    derive_ipc_inner(ast).unwrap_or_else(|e| e).into()
}

fn derive_ipc_inner(
    ast: DeriveInput,
) -> Result<proc_macro2::TokenStream, proc_macro2::TokenStream> {
    // println!("AST: {:?}", ts);
    // Ensure the thing is using a repr we support.
    ensure_valid_repr(&ast)?;

    // let try_from_bytes = derive_try_from_bytes_inner(&ast);
    let transmittable_checks = match &ast.data {
        syn::Data::Struct(r#struct) => generate_transmittable_checks_struct(&ast, r#struct)?,
        syn::Data::Enum(r#enum) => generate_transmittable_checks_enum(&ast, r#enum)?,
        syn::Data::Union(r#union) => generate_transmittable_checks_union(&ast, r#union)?,
    };
    // eprintln!("TRANSMITTABLE_CHECKS: {}", transmittable_checks);

    let padded_version = generate_padded_version(&ast)?;

    // eprintln!("PADDED_VERSION: {}", padded_version);

    // IntoIterator::into_iter([try_from_bytes, from_zeros]).collect()
    // ts
    // TokenStream::new()
    Ok(quote! {
        #transmittable_checks
        #padded_version
    })
}

fn ensure_valid_repr(ast: &DeriveInput) -> Result<(), proc_macro2::TokenStream> {
    let mut repr_c = false;
    for attr in ast.attrs.iter() {
        if attr.path().is_ident("repr") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("C") {
                    repr_c = true;
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
                }
                Ok(())
            })
            .map_err(|e| e.to_compile_error())?;
        }
    }
    if !repr_c {
        Err(syn::Error::new(ast.span(), "XousIpc only supports repr(C) structs").to_compile_error())
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
    // eprintln!("Type: {}", type_to_string(ty));

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
        .to_compile_error()),
    }
}

fn generate_transmittable_checks_enum(
    ast: &syn::DeriveInput,
    enm: &syn::DataEnum,
) -> Result<proc_macro2::TokenStream, proc_macro2::TokenStream> {
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
                Err(e) => return Err(e),
            }
        }

        variants.push(quote! {
                #(#vetted_fields)*
        });
    }

    Ok(quote! {
        #[allow(non_snake_case)]
        fn #surrounding_function () {
            pub fn ensure_is_transmittable<T: crate::IpcSafe>() {}
            #(#variants)*
        }

    })
}

fn generate_transmittable_checks_struct(
    ast: &syn::DeriveInput,
    strct: &syn::DataStruct,
) -> Result<proc_macro2::TokenStream, proc_macro2::TokenStream> {
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
            Err(e) => return Err(e),
        }
    }
    Ok(quote! {
        #[allow(non_snake_case)]
        fn #surrounding_function () {
            pub fn ensure_is_transmittable<T: crate::IpcSafe>() {}
            #(#vetted_fields)*
        }
    })
}

fn generate_transmittable_checks_union(
    ast: &syn::DeriveInput,
    unn: &syn::DataUnion,
) -> Result<proc_macro2::TokenStream, proc_macro2::TokenStream> {
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
    Ok(proc_macro2::TokenStream::new())
}

fn generate_padded_version(
    ast: &DeriveInput,
) -> Result<proc_macro2::TokenStream, proc_macro2::TokenStream> {
    let visibility = ast.vis.clone();
    let ident = ast.ident.clone();
    let padded_ident = format_ident!("Ipc{}", ast.ident);
    let ident_size = quote! { core::mem::size_of::< #ident >() };
    let padded_size = quote! { (#ident_size + (4096 - 1)) & !(4096 - 1) };
    let padding_size = quote! { #padded_size - #ident_size };
    let hash = ast_hash(ast);

    Ok(quote! {
        #[repr(C, align(4096))]
        #visibility struct #padded_ident {
            data: [u8; #padded_size],
        }

        impl core::ops::Deref for #padded_ident {
            type Target = #ident ;
            fn deref(&self) -> &Self::Target {
                unsafe {
                    let inner_ptr =
                        self.data.as_ptr() as *const [u8; core::mem::size_of::< #ident >()] as *const #ident;
                    &*inner_ptr
                }
            }
        }

        impl core::ops::DerefMut for #padded_ident {
            fn deref_mut(&mut self) -> &mut Self::Target {
                unsafe {
                    let inner_ptr =
                        self.data.as_ptr() as *mut [u8; core::mem::size_of::< #ident >()] as *mut #ident ;
                    &mut *inner_ptr
                }
            }
        }

        impl crate::IntoIpc for #ident {
            type IpcType = #padded_ident;
            fn into_ipc(self) -> Self::IpcType {
                let mut padded = #padded_ident {
                    data: [0; #padded_size],
                };
                unsafe {
                    let self_ptr = &self as *const #ident as *const [u8; #ident_size];
                    padded.data[..#ident_size].copy_from_slice(&*self_ptr);
                }
                core::mem::forget(self);
                padded
            }
        }

        impl Drop for #padded_ident {
            fn drop(&mut self) {
                unsafe {
                    let inner_ptr =
                        self.data.as_ptr() as *mut [u8; core::mem::size_of::< #ident >()] as *mut #ident ;
                    core::ptr::drop_in_place(inner_ptr);
                }
            }
        }

        impl crate::Ipc for #padded_ident {
            type Original = #ident ;

            fn from_slice<'a>(data: &'a [u8], signature: usize) -> Option<&'a Self> {
                if (data.len() < core::mem::size_of::< #padded_ident >()) {
                    return None;
                }
                if signature as u32 != #hash {
                    return None;
                }
                unsafe { Some(&*(data.as_ptr() as *const u8 as *const #padded_ident)) }
            }

            unsafe fn from_buffer_unchecked<'a>(data: &'a [u8]) -> &'a Self {
                &*(data.as_ptr() as *const u8 as *const #padded_ident)
            }

            fn from_slice_mut<'a>(data: &'a mut [u8], signature: usize) -> Option<&'a mut Self> {
                if (data.len() < core::mem::size_of::< #padded_ident >()) {
                    return None;
                }
                if signature as u32 != #hash {
                    return None;
                }
                unsafe { Some(&mut *(data.as_ptr() as *mut u8 as *mut #padded_ident)) }
            }

            unsafe fn from_buffer_mut_unchecked<'a>(data: &'a mut [u8]) -> &'a mut Self {
                unsafe { &mut *(data.as_ptr() as *mut u8 as *mut #padded_ident) }
            }

            fn lend(&self, connection: usize, opcode: usize) {
                crate::test::mock::IPC_MACHINE.lock().unwrap().lend(connection, opcode, self.signature() as usize, 0, &self.data);
            }

            fn lend_mut(&mut self, connection: usize, opcode: usize) {
                crate::test::mock::IPC_MACHINE.lock().unwrap().lend_mut(connection, opcode, self.signature() as usize, 0, &mut self.data);
            }

            fn as_original(&self) -> &Self::Original {
                unsafe {
                    &*(self.data[0.. #ident_size].as_ptr() as *const [u8; #ident_size] as *const #ident)
                }
            }

            fn as_original_mut(&mut self) -> &mut Self::Original {
                unsafe {
                    &mut *(self.data[0.. #ident_size].as_ptr() as *mut [u8; #ident_size] as *mut #ident)
                }
            }

            fn into_original(self) -> Self::Original {
                let mut original = [0u8; #ident_size];
                original.copy_from_slice(&self.data[0..#ident_size]);
                core::mem::forget(self);
                unsafe {
                    core::mem::transmute(original)
                }
            }

            fn signature(&self) -> u32 {
                #hash
            }
        }
    })
}
