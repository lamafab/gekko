pub use latest::*;

#[cfg(feature = "generator")]
pub mod generator {
    pub use project_x_generator::*;
}

pub mod latest {
    #[project_x_generator::parse_from_file("metadata/dumps/metadata_polkadot_9050.json")]
    struct RM9050;
}
