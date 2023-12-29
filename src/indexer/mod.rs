pub mod database;
pub mod inscription;

use crate::config::HttpProviders;
use crate::config::{ChainId, WsProvider, CHAINS_CONFIG};
use ethers::providers::{Http, Provider, Ws};
use log::error;
use rocksdb::{Options, TransactionDB, TransactionDBOptions, DB};
use serde::{Deserialize, Serialize};
use std::{process, sync::Arc};
use tokio::sync::Mutex;

pub const OP_MINT: &'static str = "mint";
pub const OP_DEPLOY: &'static str = "deploy";
pub const PREFIX_INSCRIPTION: &'static str = "data:,";
pub const PREFIX_INSCRIPTION_HEX: &'static str = "0x646174613a2c";
pub const DEFAULT_DB_PATH: &'static str = "./database";
pub const DEFAULT_START_TXI: i64 = -1;

lazy_static! {
    pub static ref DEFAULT_START_BLOCK: u64 = std::env::var("DEFAULT_START_BLOCK")
        .unwrap()
        .parse::<u64>()
        .unwrap();
}

pub struct Indexer {
    chain_id: ChainId,
    indexed_type: IndexedType,
    wss: WsProvider,
    https: HttpProviders,
    db: Arc<Mutex<TransactionDB>>,
}

impl Indexer {
    pub async fn new(chain_id: ChainId, indexed_type: IndexedType) -> Indexer {
        let config = CHAINS_CONFIG.get(&chain_id).unwrap();
        let https = config
            .https
            .iter()
            .map(|x| Arc::new(Provider::<Http>::try_from(x).unwrap()))
            .collect();
        let wss = Arc::new(Provider::<Ws>::new(
            Ws::connect(config.wss.as_str()).await.unwrap(),
        ));
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let txn_opts = TransactionDBOptions::default();
        let db = Arc::new(Mutex::new(
            TransactionDB::open(&opts, &txn_opts, DEFAULT_DB_PATH).unwrap(),
        ));
        Indexer {
            chain_id,
            indexed_type,
            wss,
            https,
            db,
        }
    }
    pub async fn get_indexed_block(&self, indexed_type: IndexedType) -> (u64, i64) {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let db = DB::open(&opts, DEFAULT_DB_PATH).unwrap();
        let indexed_key = format!("indexed#{}", self.chain_id);
        let indexed_value = db.get(indexed_key.as_bytes());
        if let Err(_) = indexed_value {
            error!(
                "Indexed block not found for {:?} {:?}",
                self.chain_id, indexed_type
            );
            process::exit(1);
        }
        let indexed_value = indexed_value.unwrap();
        let indexed_record: IndexedRecord;
        if let None = indexed_value {
            indexed_record = IndexedRecord {
                chain_id: self.chain_id,
                indexed_block: DEFAULT_START_BLOCK.to_owned(),
                indexed_txi: DEFAULT_START_TXI,
            };
            let indexed_value = serde_json::to_string(&indexed_record).unwrap();
            let _ = db.put(indexed_key.as_bytes(), indexed_value.as_bytes());
        } else {
            let indexed_value = indexed_value.unwrap();
            indexed_record = serde_json::from_slice(&indexed_value).unwrap();
        }
        (indexed_record.indexed_block, indexed_record.indexed_txi)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub enum IndexedType {
    TextPlain,
    ApplicationJson,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct IndexedRecord {
    pub chain_id: u64,
    pub indexed_block: u64,
    pub indexed_txi: i64,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Inscription {
    pub id: String,
    pub chain_id: u64,
    pub p: String,
    pub op: String,
    pub tick: String,
    pub max: Option<String>,
    pub lim: Option<String>,
    pub amt: Option<String>,
    pub block: u64,
    pub owner: String,
}
#[derive(Deserialize, Serialize, Debug)]
pub struct Tick {
    pub id: String,
    pub chain_id: u64,
    pub p: String,
    pub op: String,
    pub tick: String,
    pub max: Option<String>,
    pub lim: Option<String>,
    pub amt: Option<String>,
    pub start_block: u64,
    pub end_block: Option<u64>,
    pub minted: String,
    pub deployer: String,
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
