use super::Inscription;
use crate::{
    config::IdToChain,
    prisma::{self, PrismaClient},
};
use anyhow::{anyhow, Ok};
use ethers::{types::Transaction, utils::hex::ToHex};
use prisma_client_rust::bigdecimal::BigDecimal;

pub type DBInscription = prisma::inscription::Data;

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
) -> Result<prisma::inscription::Data, anyhow::Error> {
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
        .map_err(|e| anyhow!(e))
}

pub async fn dump_mint_inscription(
    db: &PrismaClient,
    tx: &Transaction,
    inp: &Inscription,
) -> Result<DBInscription, anyhow::Error> {
    let chain = tx.chain_id.unwrap().as_chain()?;
    let inscription = db
        .inscription()
        .find_unique(prisma::inscription::UniqueWhereParam::ChainPTickEquals(
            chain,
            inp.p.to_owned(),
            inp.tick.to_owned(),
        ))
        .exec()
        .await?;
    if let None = inscription {
        return Err(anyhow!("Not found deployed inscription"));
    }
    let inscription = inscription.unwrap();
    let amt = inp.amt.as_ref().unwrap().parse::<BigDecimal>().unwrap();
    let max = inscription.max.parse::<BigDecimal>().unwrap();
    let minted = inscription.minted.parse::<BigDecimal>().unwrap();
    let updated_minted = minted + amt;
    if updated_minted.gt(&max) {
        return Err(anyhow!("Max supply is reached"));
    }
    let _ = db
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
        .map_err(|e| anyhow!(e))?;
    db.inscription()
        .update(
            prisma::inscription::UniqueWhereParam::IdEquals(inscription.id),
            vec![prisma::inscription::minted::set(updated_minted.to_string())],
        )
        .exec()
        .await
        .map_err(|e| anyhow!(e))
}
