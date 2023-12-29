pub mod config;
pub mod indexer;
pub mod prisma;

use config::ChainId;
use indexer::{IndexedType, Indexer};
use tokio::join;

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
    let indexer = Indexer::new(chain_id, IndexedType::TextPlain).await;
    let _ = join!(indexer.index_inscriptions());
}
