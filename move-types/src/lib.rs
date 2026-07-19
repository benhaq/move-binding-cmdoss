pub mod functions;

use serde::Deserialize;
use serde::Serialize;
use std::fmt;
use std::str::FromStr;
pub use sui_sdk_types::Address;
pub use sui_sdk_types::Identifier;
pub use sui_sdk_types::StructTag;
pub use sui_sdk_types::TypeTag;
pub use sui_sdk_types::U256;

// ObjectId is now just an Address in sui-sdk-types, but we wrap it in a newtype
// to maintain the correct MoveType implementation (0x2::object::UID)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ObjectId(pub Address);

pub const MOVE_STDLIB: Address = {
    let mut address = [0u8; 32];
    address[31] = 1;
    Address::new(address)
};

pub trait MoveType: Serialize {
    fn type_() -> TypeTag;
}

pub trait MoveStruct: Serialize {
    fn struct_type() -> StructTag;
}

impl<T: MoveStruct> MoveType for T {
    fn type_() -> TypeTag {
        TypeTag::Struct(Self::struct_type().into())
    }
}

// todo: simplify with macros
impl MoveType for u8 {
    fn type_() -> TypeTag {
        TypeTag::U8
    }
}
impl MoveType for u16 {
    fn type_() -> TypeTag {
        TypeTag::U16
    }
}
impl MoveType for u32 {
    fn type_() -> TypeTag {
        TypeTag::U32
    }
}
impl MoveType for u64 {
    fn type_() -> TypeTag {
        TypeTag::U64
    }
}
impl MoveType for u128 {
    fn type_() -> TypeTag {
        TypeTag::U128
    }
}

impl MoveType for U256 {
    fn type_() -> TypeTag {
        TypeTag::U256
    }
}

impl MoveType for Address {
    fn type_() -> TypeTag {
        TypeTag::Address
    }
}

impl MoveType for bool {
    fn type_() -> TypeTag {
        TypeTag::Bool
    }
}

impl MoveType for ObjectId {
    fn type_() -> TypeTag {
        TypeTag::Struct(Box::new(StructTag::new(
            Address::TWO,
            Identifier::from_str("object").unwrap(),
            Identifier::from_str("UID").unwrap(),
            vec![],
        )))
    }
}

impl MoveType for String {
    fn type_() -> TypeTag {
        TypeTag::Struct(Box::new(StructTag::new(
            MOVE_STDLIB,
            Identifier::from_str("string").unwrap(),
            Identifier::from_str("String").unwrap(),
            vec![],
        )))
    }
}

impl MoveType for &str {
    fn type_() -> TypeTag {
        String::type_()
    }
}

impl<T: MoveType> MoveType for Option<T> {
    fn type_() -> TypeTag {
        TypeTag::Struct(Box::new(StructTag::new(
            MOVE_STDLIB,
            Identifier::from_str("option").unwrap(),
            Identifier::from_str("Option").unwrap(),
            vec![T::type_()],
        )))
    }
}

impl<T: MoveType> MoveType for Vec<T> {
    fn type_() -> TypeTag {
        TypeTag::Vector(Box::new(T::type_()))
    }
}

pub trait Key: MoveStruct {
    fn id(&self) -> &ObjectId;
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.to_string())
    }
}

impl From<Address> for ObjectId {
    fn from(address: Address) -> Self {
        ObjectId(address)
    }
}

impl Into<Address> for ObjectId {
    fn into(self) -> Address {
        self.0
    }
}
