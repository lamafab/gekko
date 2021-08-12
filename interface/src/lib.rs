pub use runtime::*;

#[cfg(feature = "dumps")]
/// Raw Kusama and Polkadot runtime metadata dumps.
pub mod dumps {
    pub use gekko_metadata::*;
}

#[cfg(feature = "generator")]
/// Substrate runtime metadata generator for creating Rust interfaces.
pub mod generator {
    pub use gekko_generator::*;
}

#[cfg(feature = "metadata")]
/// Utilities for parsing substrate runtime metadata.
pub mod metadata {
    pub use gekko_metadata::*;
}

pub mod transaction;
// TODO: Rename to "primitives"?
pub mod common;

pub mod runtime {
    pub mod polkadot {
        pub const LATEST_SPEC_VERSION: u32 = 9050;

        // TODO: This should use the `gekko-dumps` crate.
        #[gekko_generator::parse_from_hex_file("dumps/hex/metadata_polkadot_9050.hex")]
        struct RM9050;
    }

    pub mod kusama {
        pub const LATEST_SPEC_VERSION: u32 = 9080;

        // TODO: This should use the `gekko-dumps` crate.
        #[gekko_generator::parse_from_hex_file("dumps/hex/metadata_kusama_9080.hex")]
        struct RM9080;
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub enum Error {
    BuilderMissingField(&'static str),
}

/// Convenience function for crate internals.
// TODO: Move this to `common::crypto`
fn blake2b<T: AsRef<[u8]>>(payload: T) -> [u8; 32] {
    let mut hash = [0; 32];
    hash.copy_from_slice(blake2_rfc::blake2b::blake2b(32, &[], payload.as_ref()).as_bytes());
    hash
}
