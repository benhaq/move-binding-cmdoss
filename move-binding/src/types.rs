use crate::move_codegen::BINDING_REGISTRY;
use crate::normalized::Type;
use itertools::Itertools;
use sui_sdk_types::Address;

pub const MOVE_STDLIB: Address = {
    let mut address = [0u8; 32];
    address[31] = 1;
    Address::new(address)
};

pub trait ToRustType {
    fn to_rust_type(&self) -> String;
    fn is_ref(&self) -> bool;
    fn to_arg_type(&self) -> String;
}

impl ToRustType for Type {
    fn to_rust_type(&self) -> String {
        match self {
            Self::Bool => "bool".to_string(),
            Self::U8 => "u8".to_string(),
            Self::U16 => "u16".to_string(),
            Self::U32 => "u32".to_string(),
            Self::U64 => "u64".to_string(),
            Self::U128 => "u128".to_string(),
            Self::U256 => "move_types::U256".to_string(),
            Self::Address => "Address".to_string(),
            Self::Signer => "Address".to_string(),
            t @ Self::Datatype(_) => try_resolve_known_types(t),
            Self::Vector(t) => {
                format!("Vec<{}>", t.to_rust_type())
            }
            Self::Reference(_is_mut, t) => {
                format!("&'static {}", t.to_rust_type())
            }
            Self::TypeParameter(index) => format!("T{index}"),
        }
    }

    fn is_ref(&self) -> bool {
        matches!(self, Self::Reference(_, _))
    }

    fn to_arg_type(&self) -> String {
        match self {
            Self::Reference(is_mut, t) => {
                if *is_mut {
                    format!("MutRef<'a, {}>", t.to_rust_type())
                } else {
                    format!("Ref<'a, {}>", t.to_rust_type())
                }
            }
            _ => format!("Arg<{}>", self.to_rust_type()),
        }
    }
}

fn try_resolve_known_types(_type: &Type) -> String {
    if let Type::Datatype(datatype) = _type {
        let address = &datatype.address;
        let module = datatype.module.as_str();
        let name = datatype.name.as_str();
        let type_arguments = &datatype.type_arguments;

        match (address, module, name) {
            (&MOVE_STDLIB, "type_name", "TypeName") => "String".to_string(),
            (&MOVE_STDLIB, "string", "String") => "String".to_string(),
            (&MOVE_STDLIB, "ascii", "String") => "String".to_string(),
            (&MOVE_STDLIB, "option", "Option") => {
                format!("Option<{}>", type_arguments[0].to_rust_type())
            }

            (&Address::TWO, "object", "UID") => "ObjectId".to_string(),
            (&Address::TWO, "object", "ID") => "ObjectId".to_string(),
            _ => {
                let cache = BINDING_REGISTRY.read().unwrap();

                let package_path = cache.get(address).cloned();
                drop(cache); // Release read lock

                let type_ = if let Some(package_path) = package_path {
                    format!("{package_path}::{module}::{name}")
                } else {
                    format!("{module}::{name}")
                };

                if type_arguments.is_empty() {
                    type_
                } else {
                    format!(
                        "{type_}<{}>",
                        type_arguments.iter().map(|ty| ty.to_rust_type()).join(", ")
                    )
                }
            }
        }
    } else {
        unreachable!()
    }
}
