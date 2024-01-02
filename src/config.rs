use ethers::{
    core::rand::{seq::SliceRandom, thread_rng},
    providers::{Http, Provider, RetryClient},
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{collections::HashMap, fs, sync::Arc};

lazy_static! {
    pub static ref CHAINS_CONFIG: HashMap<ChainId, ChainConfig> =
        read_yaml::<HashMap<ChainId, ChainConfig>>("chains.config.yaml").unwrap();
}

pub type ChainId = u64;
pub type HttpProvider = Arc<Provider<RetryClient<Http>>>;
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
