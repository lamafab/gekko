use convert_case::{Case, Casing};
use proc_macro::TokenTree;
use proc_macro2::TokenStream;
use project_x_metadata::{parse_jsonrpc_metadata, ModuleMetadataExt};
use quote::{format_ident, quote};
use std::collections::HashMap;
use std::fs::read_to_string;

#[proc_macro_attribute]
pub fn from_file(
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
    println!(">> {}", path);

    // Read content from file.
    let content = read_to_string(&path).expect(&format!(
        "Failed to read runtime metadata from \"{}\"",
        path
    ));

    process_runtime_metadata(content.as_str()).into()
}

#[proc_macro_attribute]
pub fn from_rpc_endpoint(
    _args: proc_macro::TokenStream,
    _: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    unimplemented!()
}

fn process_runtime_metadata(content: &str) -> TokenStream {
    // Parse runtime metadata
    let data = parse_jsonrpc_metadata(content)
        .map_err(|err| panic!("Failed to parse runtime metadata: {:?}", err))
        .unwrap()
        .into_inner();

    let mut final_stream = TokenStream::new();
    let mut modules: HashMap<syn::Ident, TokenStream> = HashMap::new();
    let extrinsics = data.modules_extrinsics();

    for ext in extrinsics {
        if ext.args.len() > 25 {
            panic!("This macro does not support more than 25 generic variables");
        };

        // Create generics, assuming there any. E.g. `<A, B, C>`
        let generics = format!("<{}>", {
            let mut generics = ext
                .args
                .iter()
                .enumerate()
                .map(|(offset, _)| char::from_u32(65 + offset as u32).unwrap())
                .fold(String::new(), |a, b| format!("{}, {}", a, b));

            // Remove first comma, assuming generics are present.
            if !generics.is_empty() {
                generics.remove(0);
            }

            generics
        });

        // Prepare types.
        let generics: syn::Generics = syn::parse_str(&generics).unwrap();
        let ext_name = format_ident!("{}", Casing::to_case(ext.extrinsic_name, Case::Pascal));
        let ext_comments = ext.documentation;

        // Create struct fields.
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
                    #name: #ty,
                }
            });

        // Build the final type.
        let msg1 = "*Note*: This library makes no assumptions about parameter types and must be specified \
        manually as generic types. Each field contains type descriptions, as \
        provided by the runtime meatadata. See the `common` module for common types which can be used.\n";

        let msg2 = "# Documentation as provided by the runtime metadata";

        // TODO: Handle case of missing documentation?
        let ty_parent: TokenStream = quote! {
            #[doc = #msg1]
            #[doc = #msg2]
            #(#[doc = #ext_comments])*
            pub struct #ext_name #generics {
                #(#ext_args)*
            }
        };

        println!("{}", ty_parent.to_string());

        // Add created type to the corresponding module.
        modules
            .entry(format_ident!("{}", ext.module_name))
            .and_modify(|stream| {
                stream.extend(ty_parent.clone());
            })
            .or_insert(ty_parent);
    }

    // Add all modules to the final stream.
    modules.iter().for_each(|(module, stream)| {
        let stream: TokenStream = quote! {
            mod #module {
                #stream
            }
        };

        final_stream.extend(stream);
    });

    final_stream
}
