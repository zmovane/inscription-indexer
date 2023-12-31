pub mod config;
pub mod indexer;

use config::ChainId;
use indexer::{Filter, IndexedType, Indexer};
use log::error;

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
    let indexer = Indexer::new(
        chain_id,
        IndexedType::TextPlain,
        Some(Filter {
            is_self_transaction: true,
            recipient: None,
            start_block: None,
            end_block: None,
            p: None,
            tick: None,
        }),
    )
    .await;
    while let Err(e) = indexer.index_inscriptions().await {
        error!("Error: {}", e);
    }
}
