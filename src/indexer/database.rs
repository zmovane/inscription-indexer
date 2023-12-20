use super::Inscription;
use crate::prisma::{self, PrismaClient};
use anyhow::{anyhow, Ok};
use ethers::{
    types::{Transaction, U256},
    utils::hex::ToHex,
};

pub async fn update_indexed_block(
    db: &PrismaClient,
    chain: &prisma::Chain,
    indexed_block: i64,
    indexed_txi: i64,
) -> Result<(), anyhow::Error> {
    db.indexed_block()
        .upsert(
            prisma::indexed_block::UniqueWhereParam::ChainIndexedTypeEquals(
                chain.to_owned(),
                prisma::IndexedType::OrdinalsTextPlain,
            ),
            prisma::indexed_block::create(
                chain.to_owned(),
                prisma::IndexedType::OrdinalsTextPlain,
                indexed_block,
                indexed_txi,
                vec![],
            ),
            vec![],
        )
        .exec()
        .await?;
    Ok(())
}

pub async fn dump_deploy_inscription(
    db: &PrismaClient,
    tx: &Transaction,
    inp: &Inscription,
) -> Result<(), anyhow::Error> {
    let chain = tx.chain_id.unwrap().as_chain()?;
    db.inscription()
        .upsert(
            prisma::inscription::UniqueWhereParam::IdEquals(tx.hash.encode_hex()),
            prisma::inscription::create(
                tx.hash.encode_hex(),
                chain,
                tx.block_number.unwrap().as_u64() as i64,
                tx.from.encode_hex(),
                inp.p.to_owned(),
                inp.tick.to_owned(),
                inp.max.to_owned().unwrap(),
                inp.lim.to_owned().unwrap(),
                vec![],
            ),
            vec![],
        )
        .exec()
        .await
        .map(|_| ())
        .map_err(|e| anyhow!(e))
}

pub async fn dump_mint_inscription(
    client: &PrismaClient,
    tx: &Transaction,
    inp: &Inscription,
) -> Result<(), anyhow::Error> {
    let chain = tx.chain_id.unwrap().as_chain()?;
    client
        .inscribe()
        .upsert(
            prisma::inscribe::UniqueWhereParam::IdEquals(tx.hash.encode_hex()),
            prisma::inscribe::create(
                tx.hash.encode_hex(),
                prisma::inscription::UniqueWhereParam::ChainPTickEquals(
                    chain,
                    inp.p.to_owned(),
                    inp.tick.to_owned(),
                ),
                tx.block_number.unwrap().as_u64() as i64,
                inp.amt.to_owned().unwrap(),
                vec![],
            ),
            vec![],
        )
        .exec()
        .await
        .map(|_| ())
        .map_err(|e| anyhow!(e))
}

trait IdToChain {
    fn as_chain(&self) -> Result<prisma::Chain, anyhow::Error>;
}

impl IdToChain for U256 {
    fn as_chain(&self) -> Result<prisma::Chain, anyhow::Error> {
        let chain = match self.as_u128() {
            1 => prisma::Chain::EthereumMainnet,
            56 => prisma::Chain::BnbchainMainnet,
            204 => prisma::Chain::OpbnbMainnet,
            _ => return Err(anyhow!("Unknown chain")),
        };
        Ok(chain)
    }
}
