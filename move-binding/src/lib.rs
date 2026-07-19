pub mod move_codegen;
pub mod normalized;
mod package_id_resolver;
mod package_provider;
mod types;

#[derive(Copy, Clone)]
pub enum SuiNetwork {
    Mainnet,
    Testnet,
}

impl SuiNetwork {
    pub fn mvr_endpoint(&self) -> &str {
        match self {
            SuiNetwork::Mainnet => "https://mainnet.mvr.mystenlabs.com",
            SuiNetwork::Testnet => "https://testnet.mvr.mystenlabs.com",
        }
    }
    pub fn gql(&self) -> &str {
        match self {
            SuiNetwork::Mainnet => "https://graphql.mainnet.sui.io/graphql",
            SuiNetwork::Testnet => "https://graphql.testnet.sui.io/graphql",
        }
    }
}
