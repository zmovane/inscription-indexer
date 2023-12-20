pub mod database;
pub mod inscription;

use crate::prisma::{self, indexed_block, Chain, IndexedType};
use ethers::providers::{Http, Provider};
use log::error;
use prisma::PrismaClient;
use serde::{Deserialize, Serialize};
use std::{process, sync::Arc};

pub const OP_MINT: &'static str = "mint";
pub const OP_DEPLOY: &'static str = "deploy";
pub const PREFIX_INSCRIPTION: &'static str = "data:,";
pub const PREFIX_INSCRIPTION_HEX: &'static str = "0x646174613a2c";

pub struct Indexer {
    chain: Chain,
    indexed_type: IndexedType,
    provider: Arc<Provider<Http>>,
    database: PrismaClient,
}

impl Indexer {
    pub async fn new(chain: Chain, indexed_type: IndexedType, url: &str) -> Indexer {
        let provider = Arc::new(Provider::<Http>::try_from(url).unwrap());
        let database = PrismaClient::_builder().build().await.unwrap();
        Indexer {
            chain,
            indexed_type,
            provider,
            database,
        }
    }
    pub async fn get_indexed_block(&self, indexed_type: IndexedType) -> (i64, i64) {
        let block_res = self
            .database
            .indexed_block()
            .find_unique(indexed_block::chain_indexed_type(self.chain, indexed_type))
            .exec()
            .await
            .unwrap();
        if let None = block_res {
            error!(
                "Indexed block not found for {:?} {:?}",
                self.chain, indexed_type
            );
            process::exit(1);
        }
        let block = block_res.unwrap();
        (block.indexed_block, block.indexed_txi)
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Inscription {
    pub p: String,
    pub op: String,
    pub tick: String,
    pub max: Option<String>,
    pub lim: Option<String>,
    pub amt: Option<String>,
}

trait InscriptionFieldValidate {
    fn is_valid_of(&self, field: &str) -> bool;
    fn is_valid_inscription(&self) -> bool;
}

impl InscriptionFieldValidate for serde_json::Value {
    fn is_valid_of(&self, field: &str) -> bool {
        let value = self.get(field);
        value.is_some() && value.unwrap().is_string()
    }
    fn is_valid_inscription(&self) -> bool {
        if vec!["op", "p", "tick"]
            .iter()
            .any(|x| self.is_valid_of(x) == false)
        {
            return false;
        }
        let op = self.get("op").unwrap().as_str().unwrap();
        match op {
            OP_MINT => self.is_valid_of("amt"),
            OP_DEPLOY => vec!["max", "lim"].iter().all(|x| self.is_valid_of(x)),
            _ => false,
        }
    }
}
