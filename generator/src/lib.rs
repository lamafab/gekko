use convert_case::{Case, Casing};
use proc_macro::{TokenStream, TokenTree};
use project_x_metadata::{parse_jsonrpc_metadata, ModuleMetadataExt};
use quote::{format_ident, quote};
use std::fs::read_to_string;

#[proc_macro_attribute]
pub fn from_file(args: TokenStream, _: TokenStream) -> TokenStream {
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

    process_runtime_metadata(content.as_str())
}

#[proc_macro_attribute]
pub fn from_rpc_endpoint(_args: TokenStream, _: TokenStream) -> TokenStream {
    unimplemented!()
}

fn process_runtime_metadata(content: &str) -> TokenStream {
    // Parse runtime metadata
    let data = parse_jsonrpc_metadata(content)
        .map_err(|err| panic!("Failed to parse runtime metadata: {:?}", err))
        .unwrap()
        .into_inner();

    let extrinsics = data.modules_extrinsics();

    let mut full = TokenStream::new();

    for ext in extrinsics {
        if ext.args.len() > 25 {
            panic!("This macro does not support more than 25 generic variables");
        };

        // Create generics, assuming there any. E.g. `<A, B, C>`
        let mut generics = format!("<{}>", {
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
        let ext_name = format_ident!("{}", Casing::to_case(ext.name, Case::Pascal));

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
        let ty_parent: TokenStream = quote! {
            pub struct #ext_name #generics {
                #(#ext_args)*
            }
        }
        .into();

        println!("{}", ty_parent.to_string());
        full.extend(ty_parent);
    }

    full
}
