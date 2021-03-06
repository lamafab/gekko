# gekko

⚠️ This project is heavily work-in-progress and not ready for production => **API breakage expected!**

Gekko offers utilities to parse substrate metadata, generate the
corresponding Rust interfaces, create transactions and the ability to
encode/decode those transaction.

The project is split into multiple crates, although all functionality can be
exposed by just using `gekko`.
* `gekko` - Contains runtime interfaces to interact with Kusama, Polkadot
  and Westend, including creating transactions.
* `gekko-metadata` - Utilities to parse and process substrate metadata.
  * Can also be enabled in `gekko` with the `"metadata"` feature.
* `gekko-generator` - Macro to generate Rust interfaces during compile time
  based on the the parsed substrate metadata.
  * Can also be enabled in `gekko` with the `"generator"` feature.

## Interacting with the runtime

Gekko exposes multiple interfaces to interact with Kusama/Polkadot, such as
extrinsics, storage entires, events, constants and errors.

#### Disclaimer about types

This library makes no assumptions about parameter types and must be
specified manually as generic types. Each field contains a type description
which can serve as a hint on what type is being expected, as provided by the
runtime meatadata. See the [`common`] module for common types which can be
used.

### Extrinsics.

Transactions can be created by using a transaction builder from the
[`transaction`] module. The transaction formats are versioned, reflecting
the changes during Substrates history. Unless you're working with historic
data, you probably want the latest version.

Extrinsics can chosen from the [`runtime`] module and constructed
accordingly. Take a look at the [`common`] module which contains utilities
for creating transaction.

####  Example

```rust
use gekko::common::*;
use gekko::transaction::*;
use gekko::runtime::polkadot::extrinsics::balances::TransferKeepAlive;

// In this example, a random key is generated. You probably want to *import* one.
let (keypair, _) = KeyPairBuilder::<Sr25519>::generate();
let currency = BalanceBuilder::new(Currency::Polkadot);

// The destination address.
let destination =
    AccountId::from_ss58_address("12eDex4amEwj39T7Wz4Rkppb68YGCDYKG9QHhEhHGtNdDy7D")
        .unwrap();

// Send 50 DOT to the destination.
let call = TransferKeepAlive {
    dest: destination,
    value: currency.balance(50),
};

// Transaction fee.
let payment = currency.balance_as_metric(Metric::Milli, 10).unwrap();

// Build the final transaction.
let transaction: PolkadotSignedExtrinsic<_> = SignedTransactionBuilder::new()
    .signer(keypair)
    .call(call)
    .nonce(0)
    .payment(payment)
    .network(Network::Polkadot)
    .spec_version(9050)
    .build()
    .unwrap();
```

## Parsing Metadata

Gekko offers utilities that allow you to search for specific extrinsics or
to iterate through all of those.

### Example

```rust
use gekko::metadata::*;

// Parse runtime metadata
let content = std::fs::read_to_string("metadata_kusama_9080.hex").unwrap();
let data = parse_hex_metadata(content).unwrap().into_inner();

// Get information about the extrinsic.
let extr = data
    .find_module_extrinsic("Balances", "transfer_keep_alive")
    .unwrap();

assert_eq!(extr.module_id, 4);
assert_eq!(extr.dispatch_id, 3);
assert_eq!(
    extr.args,
    vec![
        ("dest", "<T::Lookup as StaticLookup>::Source"),
        ("value", "Compact<T::Balance>"),
    ]
);
```

A macro available in `gekko::generator` will parse the metadata
automatically for you and generate the Rust interfaces at compile time.

License: MIT
