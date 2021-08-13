//! Signed and unsigned transactions to be submitted to the network.
//!
//! Available implementations are versioned to reflect changes of the Substrates
//! transaction format. Unless you're dealing with historic extrinsics, you
//! probably want to use the latest version.
//!
//! The easiest way to create transactions is to use the
//! [`SignedTransactionBuilder`] type.
pub mod v4;

// Re-export the latest version.
pub use v4::{PolkadotSignedExtrinsic, SignedTransactionBuilder, Transaction};
