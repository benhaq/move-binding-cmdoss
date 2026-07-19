# Move Binding

Move Binding is a Rust library that provides a way to interact with Sui Move packages on-chain. It reads Move packages from the Sui blockchain and generates corresponding Rust structs and function entry points, allowing for seamless integration between Move and Rust. 

> ⚠️ **Disclaimer**  
> This library is experimental and may not be actively maintained or fully supported by Mysten Labs.  
> Developers should be aware that features may change without notice, and community or official support could be limited.  
> **Use at your own risk**, and thoroughly test any integration in your own development environment before relying on it in production systems.

## Features
- Reads Sui Move packages directly from the Sui blockchain.
- Generates Rust representations of Move objects.
- Provides Rust function entry points for Move contract interactions.
- Facilitates seamless integration between Move smart contracts and Rust applications.

## Installation
To use Move Binding in your project, add the following dependency to your `Cargo.toml`:

```toml
[dependencies]
move-binding-derive = { git = "https://github.com/MystenLabs/move-binding" }
move-types = { git = "https://github.com/MystenLabs/move-binding" }
```

## Usage

### Import a Move Package from Sui
```rust
use serde_json::json;
use sui_graphql::Client;

use crate::bridge::bridge::BridgeInner;
use crate::sui::dynamic_field::Field;
use move_binding_derive::move_contract;

move_contract! {alias = "sui", package = "0x2"}
move_contract! {alias = "bridge", package = "0xb"}

// Example for move package import where base module path is not "crate"
pub mod models {
    use move_binding_derive::move_contract;
    move_contract! {alias = "sui_system", package = "0x3", base_path = crate::models }
}

#[derive(serde::Deserialize)]
struct BridgeObject {
    #[serde(rename = "object.data.contents.bytes")]
    contents: Option<Vec<u8>>,
}

#[tokio::main]
async fn main() {
    let client = Client::new("https://graphql.mainnet.sui.io/graphql").unwrap();
    let response = client
        .query::<BridgeObject>(
            r#"
            query ($id: SuiAddress!) {
                object(address: $id) {
                    data {
                        ... on MoveStruct {
                            contents { bytes }
                        }
                    }
                }
            }
            "#,
            json!({ "id": "0x00ba8458097a879607d609817a05599dc3e9e73ce942f97d4f1262605a8bf0fc" }),
        )
        .await
        .unwrap();

    if let Some(contents) = response.data().and_then(|o| o.contents.as_ref()) {
        let bridge: Field<u64, BridgeInner> = bcs::from_bytes(contents).unwrap();
        println!("Deserialized Bridge object: {:?}", bridge);
    }
}
```

### Call move functions using sui-transaction-builder
```rust
use std::str::FromStr;
use sui_sdk_types::{Address, Digest};
use sui_transaction_builder::{ObjectInput, TransactionBuilder};
use move_binding_derive::move_contract;

move_contract! {alias = "sui", package = "0x2"}

fn main() {
    let owner = Address::from_str("0x2").unwrap();
    let gas_object_id = Address::from_str("0x726b714a3c4c681d8a9b1ff1833ad368585579a273362e1cbd738c0c8f70dabd").unwrap();

    let mut builder = TransactionBuilder::new();
    builder.set_sender(owner);
    builder.add_gas_objects(vec![ObjectInput::owned(gas_object_id, 1u64, Digest::from([0u8; 32]))]);
    builder.set_gas_budget(10000000);
    builder.set_gas_price(1000);

    let mut new_bag = sui::bag::new(&mut builder);
    sui::bag::add(&mut builder, new_bag.borrow_mut(), "Test".into(), "Test_value".into());
    sui::bag::add(&mut builder, new_bag.borrow_mut(), "Test2".into(), "Test_value2".into());
    sui::transfer::public_transfer(&mut builder, new_bag, owner.into());

    let tx = builder.try_build().unwrap();
    println!("{:?}", tx);
}
```


## Development
Clone the repository and build the project:

```sh
git clone https://github.com/MystenLabs/move-binding.git
cd move-binding
cargo build
```

To run tests:
```sh
cargo test
```

## Contributing
Contributions are welcome! Please open an issue or submit a pull request if you have any improvements or bug fixes.

## License
This project is licensed under the Apache 2.0 License.

---
