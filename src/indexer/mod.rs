pub mod database;
pub mod inscription;
pub mod keys;

use crate::config::{ChainId, WsProvider, CHAINS_CONFIG};
use crate::config::{HttpProviders, Random};
use ethers::providers::{Http, Middleware, Provider, Ws};
use ethers::types::{BlockNumber, H160};
use log::error;
use rocksdb::{Options, TransactionDB, TransactionDBOptions, DB};
use serde::{Deserialize, Serialize};
use std::{process, sync::Arc};
use tokio::sync::Mutex;

use self::keys::Keys;

pub const OP_MINT: &'static str = "mint";
pub const OP_DEPLOY: &'static str = "deploy";
pub const PREFIX_INSCRIPTION: &'static str = "data:,";
pub const PREFIX_INSCRIPTION_HEX: &'static str = "0x646174613a2c";
pub const DEFAULT_DB_PATH: &'static str = "./data";
pub const DEFAULT_START_TXI: i64 = -1;

lazy_static! {
    pub static ref DB_PATH: String =
        std::env::var("DB_PATH").unwrap_or(DEFAULT_DB_PATH.to_string());
}

pub struct Filter {
    pub is_self_transaction: bool,
    pub recipient: Option<H160>,
    pub start_block: Option<u64>,
    pub end_block: Option<u64>,
    pub p: Option<String>,
    pub tick: Option<String>,
}

impl Filter {
    pub fn default() -> Self {
        Filter {
            is_self_transaction: true,
            recipient: None,
            start_block: None,
            end_block: None,
            p: None,
            tick: None,
        }
    }
}

pub struct Indexer {
    chain_id: ChainId,
    indexed_type: IndexedType,
    wss: WsProvider,
    https: HttpProviders,
    db: Arc<Mutex<TransactionDB>>,
    filter: Filter,
}

impl Indexer {
    pub async fn new(chain_id: ChainId, indexed_type: IndexedType, filter: Option<Filter>) -> Self {
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
        let cfs: Vec<String> = DB::list_cf::<&str>(&opts, DB_PATH.as_str()).unwrap_or(vec![]);
        let db = Arc::new(Mutex::new(
            TransactionDB::open_cf(&opts, &txn_opts, DB_PATH.as_str(), cfs).unwrap(),
        ));
        let filter = if filter.is_some() {
            filter.unwrap()
        } else {
            Filter::default()
        };
        Indexer {
            chain_id,
            indexed_type,
            wss,
            https,
            db,
            filter,
        }
    }
    pub async fn get_indexed_block(&self, indexed_type: IndexedType) -> (u64, i64) {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let indexed_key = self.key_indexed_record();
        let indexed_value = self.db.lock().await.get(indexed_key.as_bytes());
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
            let indexed_block = if self.filter.start_block.is_some() {
                self.filter.start_block.unwrap()
            } else {
                self.https
                    .random()
                    .unwrap()
                    .get_block(BlockNumber::Latest)
                    .await
                    .unwrap()
                    .unwrap()
                    .number
                    .unwrap()
                    .as_u64()
            };
            indexed_record = IndexedRecord {
                chain_id: self.chain_id,
                indexed_block,
                indexed_txi: DEFAULT_START_TXI,
            };
            let indexed_value = serde_json::to_string(&indexed_record).unwrap();
            let _ = self
                .db
                .lock()
                .await
                .put(indexed_key.as_bytes(), indexed_value.as_bytes());
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
    pub p: String,
    pub op: String,
    pub tick: String,
    pub max: Option<String>,
    pub lim: Option<String>,
    pub amt: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DBInscription {
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
