use anyhow::{anyhow, bail};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;
use sui_sdk_types::Address;

/// Normalized representation of an on-chain Move package, built from the Sui
/// GraphQL API instead of parsing module bytecode locally.
pub struct Package {
    /// The resolved on-chain address the package was requested at.
    pub address: Address,
    pub version: u64,
    pub modules: BTreeMap<String, Module>,
    /// module name -> datatype name -> defining package id
    pub type_origin_table: HashMap<String, HashMap<String, Address>>,
}

pub struct Module {
    pub name: String,
    pub structs: Vec<Struct>,
    pub enums: Vec<Enum>,
    pub functions: Vec<Function>,
}

pub struct TypeParameter {
    pub is_phantom: bool,
    pub has_key: bool,
}

pub struct Struct {
    pub name: String,
    pub has_key_ability: bool,
    pub type_parameters: Vec<TypeParameter>,
    pub fields: Vec<Field>,
}

pub struct Enum {
    pub name: String,
    pub type_parameters: Vec<TypeParameter>,
    pub variants: Vec<Variant>,
}

pub struct Variant {
    pub name: String,
    pub fields: Vec<Field>,
}

pub struct Field {
    pub name: String,
    pub type_: Type,
}

pub struct Function {
    pub name: String,
    pub type_parameters: Vec<TypeParameter>,
    pub parameters: Vec<Type>,
    pub returns: Vec<Type>,
}

#[derive(Debug, Clone)]
pub struct Datatype {
    pub address: Address,
    pub module: String,
    pub name: String,
    pub type_arguments: Vec<Type>,
}

#[derive(Debug, Clone)]
pub enum Type {
    Bool,
    U8,
    U16,
    U32,
    U64,
    U128,
    U256,
    Address,
    Signer,
    Vector(Box<Type>),
    Datatype(Box<Datatype>),
    Reference(bool, Box<Type>),
    TypeParameter(u16),
}

impl Type {
    /// Parse a GraphQL `MoveFunctionTypeSignature`/`OpenMoveType` signature:
    /// `{ "ref": "&" | "&mut" | null, "body": <body> }`.
    pub fn from_signature(signature: &Value) -> Result<Self, anyhow::Error> {
        let body = Self::from_body(&signature["body"])?;
        Ok(match signature["ref"].as_str() {
            Some("&") => Type::Reference(false, Box::new(body)),
            Some("&mut") => Type::Reference(true, Box::new(body)),
            Some(other) => bail!("Unknown reference kind: {other}"),
            None => body,
        })
    }

    /// Parse a GraphQL type body: a primitive as a string, or one of
    /// `{"vector": <body>}`, `{"datatype": {...}}`, `{"typeParameter": n}`.
    pub fn from_body(body: &Value) -> Result<Self, anyhow::Error> {
        if let Some(s) = body.as_str() {
            return Ok(match s {
                "bool" => Type::Bool,
                "u8" => Type::U8,
                "u16" => Type::U16,
                "u32" => Type::U32,
                "u64" => Type::U64,
                "u128" => Type::U128,
                "u256" => Type::U256,
                "address" => Type::Address,
                "signer" => Type::Signer,
                other => bail!("Unknown primitive type: {other}"),
            });
        }
        if let Some(elem) = body.get("vector") {
            return Ok(Type::Vector(Box::new(Self::from_body(elem)?)));
        }
        if let Some(datatype) = body.get("datatype") {
            let address = Address::from_str(
                datatype["package"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing datatype package: {datatype}"))?,
            )?;
            let module = datatype["module"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing datatype module: {datatype}"))?
                .to_string();
            let name = datatype["type"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing datatype name: {datatype}"))?
                .to_string();
            let type_arguments = datatype["typeParameters"]
                .as_array()
                .map(|args| args.iter().map(Self::from_body).collect::<Result<_, _>>())
                .transpose()?
                .unwrap_or_default();
            return Ok(Type::Datatype(Box::new(Datatype {
                address,
                module,
                name,
                type_arguments,
            })));
        }
        if let Some(index) = body.get("typeParameter") {
            let index = index
                .as_u64()
                .ok_or_else(|| anyhow!("Invalid type parameter index: {index}"))?;
            return Ok(Type::TypeParameter(u16::try_from(index)?));
        }
        bail!("Unknown type body: {body}")
    }
}

fn parse_type_parameter(v: &Value) -> TypeParameter {
    TypeParameter {
        is_phantom: v["isPhantom"].as_bool().unwrap_or(false),
        has_key: v["constraints"]
            .as_array()
            .map(|c| c.iter().any(|a| a.as_str() == Some("KEY")))
            .unwrap_or(false),
    }
}

fn parse_type_parameters(v: &Value) -> Vec<TypeParameter> {
    v.as_array()
        .map(|params| params.iter().map(parse_type_parameter).collect())
        .unwrap_or_default()
}

fn parse_fields(v: &Value) -> Result<Vec<Field>, anyhow::Error> {
    v.as_array()
        .map(|fields| {
            fields
                .iter()
                .map(|field| {
                    Ok::<_, anyhow::Error>(Field {
                        name: field["name"]
                            .as_str()
                            .ok_or_else(|| anyhow!("Missing field name: {field}"))?
                            .to_string(),
                        type_: Type::from_signature(&field["type"]["signature"])?,
                    })
                })
                .collect()
        })
        .transpose()?
        .ok_or_else(|| anyhow!("Missing fields: {v}"))
}

impl Struct {
    pub fn from_node(node: &Value) -> Result<Self, anyhow::Error> {
        Ok(Struct {
            name: node["name"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing struct name: {node}"))?
                .to_string(),
            has_key_ability: node["abilities"]
                .as_array()
                .map(|a| a.iter().any(|v| v.as_str() == Some("KEY")))
                .unwrap_or(false),
            type_parameters: parse_type_parameters(&node["typeParameters"]),
            fields: parse_fields(&node["fields"])?,
        })
    }
}

impl Enum {
    pub fn from_node(node: &Value) -> Result<Self, anyhow::Error> {
        let variants = node["variants"]
            .as_array()
            .map(|variants| {
                variants
                    .iter()
                    .map(|variant| {
                        Ok::<_, anyhow::Error>(Variant {
                            name: variant["name"]
                                .as_str()
                                .ok_or_else(|| anyhow!("Missing variant name: {variant}"))?
                                .to_string(),
                            fields: parse_fields(&variant["fields"])?,
                        })
                    })
                    .collect::<Result<_, _>>()
            })
            .transpose()?
            .unwrap_or_default();
        Ok(Enum {
            name: node["name"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing enum name: {node}"))?
                .to_string(),
            type_parameters: parse_type_parameters(&node["typeParameters"]),
            variants,
        })
    }
}

impl Function {
    pub fn from_node(node: &Value) -> Result<Self, anyhow::Error> {
        let parse_signatures = |v: &Value| {
            v.as_array()
                .map(|types| {
                    types
                        .iter()
                        .map(|t| Type::from_signature(&t["signature"]))
                        .collect::<Result<Vec<_>, _>>()
                })
                .transpose()
                .map(Option::unwrap_or_default)
        };
        Ok(Function {
            name: node["name"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing function name: {node}"))?
                .to_string(),
            type_parameters: parse_type_parameters(&node["typeParameters"]),
            parameters: parse_signatures(&node["parameters"])?,
            returns: parse_signatures(&node["return"])?,
        })
    }
}
