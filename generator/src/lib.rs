use convert_case::{Case, Casing};
use gekko_metadata::{parse_hex_metadata, ModuleMetadataExt};
use proc_macro::TokenTree;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::HashMap;
use std::fs::read_to_string;

#[proc_macro_attribute]
pub fn parse_from_hex_file(
    args: proc_macro::TokenStream,
    _: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    // Extract path.
    let tree = args
        .into_iter()
        .nth(0)
        .expect("Expected path literal as argument. E.g \"/path/to/file\"");

    let path = match tree {
        TokenTree::Literal(path) => path.to_string(),
        _ => panic!("Expected path literal as argument. E.g \"/path/to/file\""),
    };

    let path = path.replace("\"", "");

    // Read content from file.
    let content = read_to_string(&path).expect(&format!(
        "Failed to read runtime metadata from \"{}\"",
        path
    ));

    process_runtime_metadata(content.as_str()).into()
}

fn process_runtime_metadata(content: &str) -> TokenStream {
    // Parse runtime metadata
    let data = parse_hex_metadata(content)
        .map_err(|err| panic!("Failed to parse runtime metadata: {:?}", err))
        .unwrap()
        .into_inner();

    let mut final_extrinsics = TokenStream::new();
    let mut modules: HashMap<syn::Ident, TokenStream> = HashMap::new();
    let extrinsics = data.modules_extrinsics();

    for ext in extrinsics {
        if ext.args.len() > 25 {
            panic!("This macro does not support more than 25 generic variables");
        };

        // Create generics, assuming there any. E.g. `<A, B, C>`
        let generics: Vec<String> = ext
            .args
            .iter()
            .enumerate()
            .map(|(offset, _)| char::from_u32(65 + offset as u32).unwrap().into())
            .collect();

        let generics_wrapped = format!("<{}>", {
            let mut generics = generics
                .iter()
                .fold(String::new(), |a, b| format!("{}, {}", a, b));

            // Remove first comma, assuming generics are present.
            if !generics.is_empty() {
                generics.remove(0);
            }

            generics
        });

        // Prepare types.
        let generics_wrapped: syn::Generics = syn::parse_str(&generics_wrapped).unwrap();
        let ext_name = format_ident!("{}", Casing::to_case(ext.extrinsic_name, Case::Pascal));
        let ext_comments: Vec<String> = ext
            .documentation
            .iter()
            .map(|doc| doc.replace("[`", "`").replace("`]", "`"))
            .collect();

        // Create individual struct fields.
        let ext_args = ext
            .args
            .iter()
            .enumerate()
            .map(|(offset, (name, ty_desc))| {
                let msg = format!("Type description: `{}`", ty_desc);
                let name = format_ident!("{}", name);
                let ty = format_ident!("{}", char::from_u32(65 + offset as u32).unwrap());
                quote! {
                    #[doc = #msg]
                    pub #name: #ty,
                }
            });

        // Specialized struct field parsing used for the `parity_scale_codec::Decode` implementation.
        let ext_args_decode = ext.args.iter().map(|(name, _)| {
            let name = format_ident!("{}", name);
            quote! {
                #name: parity_scale_codec::Decode::decode(input)?,
            }
        });

        // Prepare documentation for type.
        let disclaimer = "# Type Disclaimer\nThis library makes no assumptions about parameter types and must be specified \
        manually as generic types. Each field contains type descriptions, as \
        provided by the runtime meatadata. See the [`common`](crate::common) module for common types which can be used.\n";

        let docs = if !ext_comments.is_empty() {
            let intro = ext_comments.iter().nth(0).unwrap();
            let msg = "# Documentation (provided by the runtime metadata)";

            quote! {
                #[doc = #intro]
                #[doc = #msg]
                #(#[doc = #ext_comments])*
            }
        } else {
            let msg = "No documentation provided by the runtime metadata";
            quote! {
                #[doc = #msg]
            }
        };

        // Build the final type.
        let generics_idents: Vec<syn::Ident> =
            generics.iter().map(|v| format_ident!("{}", v)).collect();

        // Enums have a max size of 256. This is acknowledged in the SCALE specification.
        let ext_module_id = ext.module_id as u8;
        let ext_dispatch_id = ext.dispatch_id as u8;

        let type_stream: TokenStream = quote! {
            #docs
            #[doc = #disclaimer]
            #[derive(Debug, Clone, Eq, PartialEq)]
            pub struct #ext_name #generics_wrapped
            where
                #(#generics_idents: parity_scale_codec::Encode + parity_scale_codec::Decode, )*
            {
                #(#ext_args)*
            }

            impl #generics_wrapped parity_scale_codec::Encode for #ext_name #generics_wrapped
            where
                #(#generics_idents: parity_scale_codec::Encode + parity_scale_codec::Decode, )*
            {
                fn using_encoded<SR, SF: FnOnce(&[u8]) -> SR>(&self, f: SF) -> SR {
                    f(&[#ext_module_id, #ext_dispatch_id])
                }
            }

            impl #generics_wrapped parity_scale_codec::Decode for #ext_name #generics_wrapped
            where
                #(#generics_idents: parity_scale_codec::Encode + parity_scale_codec::Decode, )*
            {
                fn decode<SI: parity_scale_codec::Input>(input: &mut SI) -> Result<Self, parity_scale_codec::Error> {
                    let mut buffer = [0; 2];
                    input.read(&mut buffer)?;

                    if buffer != [#ext_module_id, #ext_dispatch_id] {
                        return Err("Invalid identifier of the expected type.".into())
                    }

                    Ok(
                        #ext_name {
                            #(#ext_args_decode )*
                        }
                    )
                }
            }
        };

        // Add created type to the corresponding module.
        modules
            .entry(format_ident!(
                "{}",
                Casing::to_case(ext.module_name, Case::Snake)
            ))
            .and_modify(|stream| {
                stream.extend(type_stream.clone());
            })
            .or_insert(type_stream);
    }

    // Add all modules to the final stream.
    modules.iter().for_each(|(module, stream)| {
        let stream: TokenStream = quote! {
            pub mod #module {
                #stream
            }
        };

        final_extrinsics.extend(stream);
    });

    quote! {
        pub mod extrinsics {
            #final_extrinsics
        }

        /// TODO
        pub mod storage {}
        /// TODO
        pub mod events {}
        /// TODO
        pub mod constants {}
        /// TODO
        pub mod errors {}
    }
}
