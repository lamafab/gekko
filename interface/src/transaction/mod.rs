pub mod version;

// Re-export the latest version.
pub use version::v4::{SignedExtrinsic, PolkadotSignedExtrinsic, ExtrinsicBuilder};
