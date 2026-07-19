use crate::bridge::bridge::BridgeInner;
use crate::sui::dynamic_field::Field;
use move_binding_derive::move_contract;
use move_types::Key;
use serde_json::json;
use std::str::FromStr;
use sui_graphql::{Client, Error};
use sui_sdk_types::Address;
use sui_transaction_builder::{ObjectInput, TransactionBuilder};

move_contract! {alias = "move_lib", package = "0x1"}
move_contract! {alias = "sui", package = "0x2"}
move_contract! {alias = "sui_system", package = "0x3"}
move_contract! {alias = "bridge", package = "0xb"}

move_contract! {alias = "mvr_metadata", package = "@mvr/metadata"}
move_contract! {alias = "suins", package = "0xd22b24490e0bae52676651b4f56660a5ff8022a2576e0089f79b3c88d44e08f0"}
move_contract! {alias = "mvr_core", package = "@mvr/core"}

move_contract! {alias = "token", package = "0xdeeb7a4662eec9f2f3def03fb937a663dddaa2e215b8078a284d026b7946c270"}

move_contract! {alias = "deepbook", package = "@deepbook/core"}

move_contract! {alias = "tokenv2", package = "0x36dbef866a1d62bf7328989a10fb2f07d769f4ee587c0de4a0a256e57e0a58a8", network="testnet"}
move_contract! {alias = "deepbookv2", package = "@deepbook/core", network = "testnet"}

//move_contract! {alias = "mvr_metadata_testnet", package = "@mvr/metadata", network = "testnet", deps = [crate::sui]}

move_contract! {alias = "commander", package = "0xdc931e30acc15dbc7fcbd39cd385a03894a7236761490ff4d5b9dbf51af3ce26", network="testnet"}

move_contract! {alias = "wal", package = "0x8270feb7375eee355e64fdb69c50abb6b5f9393a722883c1cf45f8e26048810a", network="testnet"}
move_contract! {alias = "walrus", package = "0xd84704c17fc870b8764832c535aa6b11f21a95cd6f5bb38a9b07d2cf42220c66", network="testnet"}
move_contract! {alias="smashblob_game", package="0x546033753470885503baf885c7d343e55678c850439de45d6e7c8f384d2e8080", network = "testnet"}

#[derive(serde::Deserialize)]
struct BridgeObject {
    #[serde(rename = "object.data.contents.bytes")]
    contents: Option<Vec<u8>>,
    #[serde(rename = "object.objectId")]
    #[allow(dead_code)]
    object_id: Option<String>,
}

#[tokio::test]
pub async fn test_deserialize_object() -> Result<(), Error> {
    let client = Client::new("https://graphql.mainnet.sui.io/graphql")?;
    let response = client
        .query::<BridgeObject>(
            r#"
            query ($id: SuiAddress!) {
                object(address: $id) {
                    objectId
                    data {
                        ... on MoveStruct {
                            contents {
                                bytes
                            }
                        }
                    }
                }
            }
            "#,
            json!({ "id": "0x00ba8458097a879607d609817a05599dc3e9e73ce942f97d4f1262605a8bf0fc" }),
        )
        .await?;

    if let Some(obj) = response.data() {
        if let Some(ref contents) = obj.contents {
            let bridge: Field<u64, BridgeInner> = bcs::from_bytes(contents).unwrap();
            println!("{:?}", bridge);
            println!("{:?}", bridge.id());
        }
    }

    Ok(())
}

#[tokio::test]
pub async fn test_function_call() -> Result<(), Error> {
    let owner = Address::from_str("0x2").unwrap();

    // Create a mock gas object with test values
    let gas_object_id =
        Address::from_str("0x726b714a3c4c681d8a9b1ff1833ad368585579a273362e1cbd738c0c8f70dabd")
            .unwrap();
    let gas_version = 1u64;
    let gas_digest = sui_sdk_types::Digest::from([0u8; 32]);

    let mut builder = TransactionBuilder::new();
    builder.set_sender(owner);
    builder.add_gas_objects(vec![ObjectInput::owned(
        gas_object_id,
        gas_version,
        gas_digest,
    )]);
    builder.set_gas_budget(10000000);
    builder.set_gas_price(1000);

    let mut new_bag = sui::bag::new(&mut builder);

    let option = move_lib::option::some(&mut builder, "Test".into());

    sui::bag::add(&mut builder, new_bag.borrow_mut(), "Test".into(), option);
    sui::bag::add(
        &mut builder,
        new_bag.borrow_mut(),
        "Test2".into(),
        "Test_value2".into(),
    );
    sui::transfer::public_transfer(&mut builder, new_bag, owner.into());

    let _tx = builder.try_build().unwrap();
    println!("Transaction built successfully");

    Ok(())
}

#[tokio::test]
async fn test_deserialize_enum() {
    use commander::history;

    // Struct Variant
    let attack = history::Record::Attack {
        origin: vec![1, 1],
        target: vec![2, 7],
    };
    let bytes = bcs::to_bytes(&attack).unwrap();
    let deserialized: history::Record = bcs::from_bytes(&bytes).unwrap();
    println!("Deserialized: {:?}", deserialized);

    // Tuple variant
    let reload = history::Record::Reload(vec![4, 7, 8, 22]);
    let bytes = bcs::to_bytes(&reload).unwrap();
    let deserialized: history::Record = bcs::from_bytes(&bytes).unwrap();
    println!("Deserialized: {:?}", deserialized);

    // Unit variant
    let miss = history::Record::Miss;
    let bytes = bcs::to_bytes(&miss).unwrap();
    let deserialized: history::Record = bcs::from_bytes(&bytes).unwrap();
    println!("Deserialized: {:?}", deserialized);
}
