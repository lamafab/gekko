use proc_macro::{TokenStream, TokenTree};
use project_x_metadata::{parse_jsonrpc_metadata, MetadataVersion};
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

    // Read content from file.
    let content = read_to_string(&path).expect(&format!(
        "Failed to read runtime metadata from \"{}\"",
        path
    ));

    // Parse runtime metadata
    let metadata = parse_jsonrpc_metadata(content)
        .map_err(|err| panic!("Failed to parse runtime metadata: {:?}", err))
        .unwrap();

    process_runtime_metadata(metadata)
}

#[proc_macro_attribute]
pub fn from_web(args: TokenStream, _: TokenStream) -> TokenStream {
    unimplemented!()
}

fn process_runtime_metadata(metadata: MetadataVersion) -> TokenStream {
    unimplemented!()
}
