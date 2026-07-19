use crate::normalized::{Enum, Function, Module, Package, Struct};
use crate::package_id_resolver::PackageIdResolver;
use crate::SuiNetwork;
use anyhow::anyhow;
use reqwest::blocking::Client;
use reqwest::header::CONTENT_TYPE;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;
use sui_sdk_types::Address;

const INNER_PAGE_SIZE: u32 = 50;
const MODULE_PAGE_SIZE: u32 = 10;

const MODULE_FIELDS: &str = r#"
    name
    structs(first: $first, after: $cursor) {
        pageInfo { hasNextPage endCursor }
        nodes {
            name
            abilities
            typeParameters { isPhantom constraints }
            fields { name type { signature } }
        }
    }
    enums(first: $first, after: $cursor) {
        pageInfo { hasNextPage endCursor }
        nodes {
            name
            abilities
            typeParameters { isPhantom constraints }
            variants { name fields { name type { signature } } }
        }
    }
    functions(first: $first, after: $cursor) {
        pageInfo { hasNextPage endCursor }
        nodes {
            name
            visibility
            isEntry
            typeParameters { constraints }
            parameters { signature }
            return { signature }
        }
    }
"#;

pub trait ModuleProvider {
    fn get_package(&self, package_id: &str) -> Result<Package, anyhow::Error>;
}

pub struct MoveModuleProvider {
    network: SuiNetwork,
}

impl MoveModuleProvider {
    pub fn new(network: SuiNetwork) -> Self {
        Self { network }
    }

    fn query(
        &self,
        client: &Client,
        query: String,
        variables: Value,
    ) -> Result<Value, anyhow::Error> {
        let res = client
            .post(self.network.gql())
            .header(CONTENT_TYPE, "application/json")
            .json(&json!({
                "query": query,
                "variables": variables
            }))
            .send()
            .map_err(|e| anyhow!("Error fetching package from Sui GQL: {e}"))?;
        let value = res.json::<Value>()?;
        if let Some(errors) = value.get("errors") {
            if !errors.is_null() {
                return Err(anyhow!("Sui GQL returned errors: {errors}"));
            }
        }
        Ok(value)
    }

    /// Fetch the remaining pages of one connection (`structs`, `enums` or
    /// `functions`) of a single module.
    fn fetch_remaining(
        &self,
        client: &Client,
        package_id: &Address,
        module_name: &str,
        connection: &str,
        selection: &str,
        mut cursor: String,
    ) -> Result<Vec<Value>, anyhow::Error> {
        let mut nodes = vec![];
        loop {
            let query = format!(
                r#"query ($addr: SuiAddress!, $module: String!, $after: String) {{
                    package(address: $addr) {{
                        module(name: $module) {{
                            {connection}(first: {INNER_PAGE_SIZE}, after: $after) {{
                                pageInfo {{ hasNextPage endCursor }}
                                nodes {{ {selection} }}
                            }}
                        }}
                    }}
                }}"#
            );
            let value = self.query(
                client,
                query,
                json!({
                    "addr": package_id.to_string(),
                    "module": module_name,
                    "after": cursor,
                }),
            )?;
            let page = &value["data"]["package"]["module"][connection];
            nodes.extend(page["nodes"].as_array().cloned().unwrap_or_default());
            if !page["pageInfo"]["hasNextPage"].as_bool().unwrap_or(false) {
                return Ok(nodes);
            }
            cursor = page["pageInfo"]["endCursor"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing endCursor for {module_name}.{connection}"))?
                .to_string();
        }
    }

    /// Collect all nodes of an inline connection, following pagination through
    /// per-module queries when a connection has more than one page.
    fn collect_connection(
        &self,
        client: &Client,
        package_id: &Address,
        module_name: &str,
        module: &Value,
        connection: &str,
        selection: &str,
    ) -> Result<Vec<Value>, anyhow::Error> {
        let conn = &module[connection];
        let mut nodes = conn["nodes"].as_array().cloned().unwrap_or_default();
        if conn["pageInfo"]["hasNextPage"].as_bool().unwrap_or(false) {
            let cursor = conn["pageInfo"]["endCursor"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing endCursor for {module_name}.{connection}"))?
                .to_string();
            nodes.extend(self.fetch_remaining(
                client,
                package_id,
                module_name,
                connection,
                selection,
                cursor,
            )?);
        }
        Ok(nodes)
    }
}

impl ModuleProvider for MoveModuleProvider {
    fn get_package(&self, package: &str) -> Result<Package, anyhow::Error> {
        let package_id = PackageIdResolver::resolve_package_id(self.network, package)?;
        let client = Client::new();

        // Fetch package metadata: version and the type origin table.
        let value = self.query(
            &client,
            r#"query ($addr: SuiAddress!) {
                package(address: $addr) {
                    version
                    typeOrigins { module struct definingId }
                }
            }"#
            .to_string(),
            json!({ "addr": package_id.to_string() }),
        )?;

        if value["data"]["package"].is_null() {
            return Err(anyhow!(
                "Package {} not found on {}. The package may not exist or has been removed.",
                package_id,
                self.network.gql()
            ));
        }

        let version = serde_json::from_value(value["data"]["package"]["version"].clone())?;

        let type_origin_table: Vec<Value> =
            serde_json::from_value(value["data"]["package"]["typeOrigins"].clone())?;

        let type_origin_table = type_origin_table.iter().fold(
            HashMap::new(),
            |mut results: HashMap<String, HashMap<String, Address>>, v| {
                // Skip entries with null values
                if let (Some(module), Some(struct_), Some(defining_id)) = (
                    v["module"].as_str(),
                    v["struct"].as_str(),
                    v["definingId"].as_str(),
                ) {
                    if let Ok(addr) = Address::from_str(defining_id) {
                        results
                            .entry(module.to_string())
                            .or_default()
                            .insert(struct_.to_string(), addr);
                    }
                }
                results
            },
        );

        // Fetch all modules with their normalized structs, enums and functions.
        let module_fields = MODULE_FIELDS
            .replace("$first", &INNER_PAGE_SIZE.to_string())
            .replace("$cursor", "null");
        let modules_query = format!(
            r#"query ($addr: SuiAddress!, $after: String) {{
                package(address: $addr) {{
                    modules(first: {MODULE_PAGE_SIZE}, after: $after) {{
                        pageInfo {{ hasNextPage endCursor }}
                        nodes {{ {module_fields} }}
                    }}
                }}
            }}"#
        );

        let mut modules = BTreeMap::new();
        let mut cursor = Value::Null;
        loop {
            let value = self.query(
                &client,
                modules_query.clone(),
                json!({ "addr": package_id.to_string(), "after": cursor }),
            )?;
            let page = &value["data"]["package"]["modules"];
            for module in page["nodes"].as_array().cloned().unwrap_or_default() {
                let module_name = module["name"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing module name"))?
                    .to_string();

                let structs = self
                    .collect_connection(
                        &client,
                        &package_id,
                        &module_name,
                        &module,
                        "structs",
                        "name abilities typeParameters { isPhantom constraints } fields { name type { signature } }",
                    )?
                    .iter()
                    .map(Struct::from_node)
                    .collect::<Result<_, _>>()?;

                let enums = self
                    .collect_connection(
                        &client,
                        &package_id,
                        &module_name,
                        &module,
                        "enums",
                        "name abilities typeParameters { isPhantom constraints } variants { name fields { name type { signature } } }",
                    )?
                    .iter()
                    .map(Enum::from_node)
                    .collect::<Result<_, _>>()?;

                let functions = self
                    .collect_connection(
                        &client,
                        &package_id,
                        &module_name,
                        &module,
                        "functions",
                        "name visibility isEntry typeParameters { constraints } parameters { signature } return { signature }",
                    )?
                    .iter()
                    .map(Function::from_node)
                    .collect::<Result<_, _>>()?;

                modules.insert(
                    module_name.clone(),
                    Module {
                        name: module_name,
                        structs,
                        enums,
                        functions,
                    },
                );
            }
            if !page["pageInfo"]["hasNextPage"].as_bool().unwrap_or(false) {
                break;
            }
            cursor = page["pageInfo"]["endCursor"].clone();
        }

        Ok(Package {
            address: package_id,
            version,
            modules,
            type_origin_table,
        })
    }
}
