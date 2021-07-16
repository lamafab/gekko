pub use latest::*;

#[cfg(feature = "generator")]
pub mod generator {
    pub use project_x_generator::*;
}

#[cfg(feature = "metadata")]
pub mod metadata {
    pub use project_x_metadata::*;
}

pub mod latest {
    #[project_x_generator::parse_from_file("metadata/dumps/metadata_polkadot_9050.json")]
    struct RM9050;
}

pub mod common {
    /// Re-export of the [`parity-scale-codec`](https://crates.io/crates/parity-scale-codec) crate.
    // TODO: Enable via feature?
    pub mod scale {
        pub use parity_scale_codec::*;
    }
}
