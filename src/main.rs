pub mod config;
pub mod indexer;
pub mod utils;

use config::ChainId;
use indexer::{Filter, IndexedType, Indexer};
use log::{error, info};

#[macro_use]
extern crate lazy_static;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    pretty_env_logger::init_timed();
    let chain_id = std::env::var("CHAIN_ID")
        .expect("CHAIN_ID must be set")
        .parse::<ChainId>()
        .unwrap();
    let mut filter: Option<Filter> = None;
    if let Ok(value) = std::env::var("START_BLOCK") {
        let start_block = value.parse::<u64>().unwrap();
        filter = Some(Filter {
            is_self_transaction: true,
            recipient: None,
            start_block: Some(start_block),
            end_block: None,
            p: None,
            tick: None,
        });
    }
    let indexer = Indexer::new(chain_id, IndexedType::TextPlain, filter).await;
    loop {
        match indexer.index_inscriptions().await {
            Err(e) => error!("Error: {}", e),
            Ok(_) => info!("Pending new block"),
        }
    }
}
