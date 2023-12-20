pub mod indexer;
pub mod prisma;
use indexer::Indexer;
use tokio::join;

#[tokio::main]
async fn main() {
    pretty_env_logger::init_timed();
    let rpc_url = std::env::var("RPC_URL").expect("RPC_URL must be set");
    let indexer = Indexer::new(
        prisma::Chain::BnbchainMainnet,
        prisma::IndexedType::OrdinalsTextPlain,
        &rpc_url,
    )
    .await;
    let _ = join!(indexer.index_inscriptions());
}
