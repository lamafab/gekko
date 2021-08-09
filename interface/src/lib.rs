pub use runtime::*;

#[cfg(feature = "generator")]
pub mod generator {
    pub use gekko_generator::*;
}

#[cfg(feature = "metadata")]
pub mod metadata {
    pub use gekko_metadata::*;
}

pub mod extrinsic;
// TODO: Rename to "primitives"?
pub mod common;

pub mod runtime {
    #[gekko_generator::parse_from_hex_file("dumps/hex/metadata_polkadot_9050.hex")]
    struct RM9050;
}

pub type Result<T> = std::result::Result<T, Error>;

pub enum Error {
    BuilderErrorContradictingEntries(&'static str, &'static str),
    BuilderErrorMissingField(String),
}
