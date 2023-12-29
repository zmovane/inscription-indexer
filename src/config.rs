use crate::prisma;
use anyhow::anyhow;
use ethers::{
    core::rand::{seq::SliceRandom, thread_rng},
    providers::{Http, Provider, Ws},
    types::U256,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{collections::HashMap, fs, sync::Arc};

lazy_static! {
    pub static ref CHAINS_CONFIG: HashMap<ChainId, ChainConfig> =
        read_yaml::<HashMap<ChainId, ChainConfig>>("chains.config.yaml").unwrap();
}

pub type ChainId = u64;
pub type WsProvider = Arc<Provider<Ws>>;
pub type HttpProvider = Arc<Provider<Http>>;
pub type HttpProviders = Vec<HttpProvider>;

pub fn read_yaml<T: DeserializeOwned>(path: &str) -> Result<T, serde_yaml::Error> {
    let content = fs::read_to_string(path).unwrap();
    let result = serde_yaml::from_str(content.as_str())?;
    Ok(result)
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ChainConfig {
    pub name: String,
    pub wss: String,
    pub https: Vec<String>,
}

pub trait Random<T> {
    fn random(&self) -> Result<T, anyhow::Error>;
}

impl Random<HttpProvider> for HttpProviders {
    fn random(&self) -> Result<HttpProvider, anyhow::Error> {
        Ok(self.choose(&mut thread_rng()).unwrap().to_owned())
    }
}

pub trait IdToChain {
    fn as_chain(&self) -> Result<prisma::Chain, anyhow::Error>;
}

impl IdToChain for U256 {
    fn as_chain(&self) -> Result<prisma::Chain, anyhow::Error> {
        self.as_u64().as_chain()
    }
}

impl IdToChain for ChainId {
    fn as_chain(&self) -> Result<prisma::Chain, anyhow::Error> {
        let chain = match self {
            1 => prisma::Chain::EthereumMainnet,
            56 => prisma::Chain::BnbchainMainnet,
            204 => prisma::Chain::OpbnbMainnet,
            _ => return Err(anyhow!("Unknown chain")),
        };
        Ok(chain)
    }
}
