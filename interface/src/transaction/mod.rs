//! Signed and unsigned transactions to be submitted to the network.
//!
//! Available implementations are versioned to reflect changes of the Substrates
//! transaction format. Unless you're dealing with historic extrinsics, you
//! probably want to use the latest version.
//!
//! The easiest way to create transactions is to use the
//! [`SignedTransactionBuilder`] type.

// Re-export the latest version.
pub use v4::{PolkadotSignedExtrinsic, SignedTransactionBuilder, Transaction};

// Version 4 of the transaction format.
pub mod v4;

/// TODO.
pub mod v5 {}
/// TODO.
pub mod v3 {}
/// TODO.
pub mod v2 {}
/// TODO.
pub mod v1 {}
