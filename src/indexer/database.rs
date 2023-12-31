use super::keys::Keys;
use super::{DBInscription, IndexedRecord, Inscription};
use super::{Indexer, Tick};
use anyhow::Ok;
use async_trait::async_trait;
use bigdecimal::BigDecimal;
use ethers::types::{Block, H256};
use ethers::{abi::AbiEncode, types::Transaction};
use log::warn;
use rocksdb::{Options, TransactionDB};

#[async_trait]
pub trait Persistable {
    async fn persist_deploy(
        &self,
        block: &Block<H256>,
        tx: &Transaction,
        inp: &Inscription,
    ) -> Result<(), anyhow::Error>;
    async fn persist_mint(
        &self,
        block: &Block<H256>,
        tx: &Transaction,
        inp: &Inscription,
    ) -> Result<(), anyhow::Error>;
    fn persist_block(
        &self,
        txn: &rocksdb::Transaction<TransactionDB>,
        indexed_block: u64,
        indexed_txi: i64,
    ) -> Result<(), anyhow::Error>;
}

#[async_trait]
impl Persistable for Indexer {
    async fn persist_deploy(
        &self,
        _: &Block<H256>,
        tx: &Transaction,
        inp: &Inscription,
    ) -> Result<(), anyhow::Error> {
        let db = self.db.lock().await;
        let chain_id = tx.chain_id.unwrap().as_u64();
        let start_block = tx.block_number.unwrap().as_u64();
        let id: String = tx.hash.encode_hex();
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        let tick_key = self.key_tick_deployed(&inp.p, &inp.tick);
        let bs = db.get(tick_key.as_bytes())?;
        if let Some(_) = bs {
            warn!("The tick has been deployed, just skip it!");
            return Ok(());
        }
        let txn = db.transaction();

        // deploy
        let tick = Tick {
            id,
            chain_id,
            start_block,
            end_block: None,
            p: inp.p.to_owned(),
            op: inp.op.to_owned(),
            tick: inp.tick.to_owned(),
            max: inp.max.to_owned(),
            lim: inp.lim.to_owned(),
            amt: inp.amt.to_owned(),
            minted: String::from("0"),
            deployer: tx.from.encode_hex(),
        };
        let tick_value = serde_json::to_string(&tick).unwrap();
        txn.put(tick_key.as_bytes(), tick_value.as_bytes())?;

        // index block
        self.persist_block(
            &txn,
            start_block,
            tx.transaction_index.unwrap().as_u64() as i64,
        )?;

        txn.commit()?;
        Ok(())
    }

    async fn persist_mint(
        &self,
        block: &Block<H256>,
        tx: &Transaction,
        inp: &Inscription,
    ) -> Result<(), anyhow::Error> {
        let db = self.db.lock().await;
        let chain_id = tx.chain_id.unwrap().as_u64();
        let blockno = tx.block_number.unwrap().as_u64();
        let id: String = tx.hash.encode_hex();

        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let tick_key = self.key_tick_deployed(&inp.p, &inp.tick);
        let bs = db.get(tick_key.as_bytes())?;
        if let None = bs {
            warn!("Not found for deployed tick, just skip it!");
            return Ok(());
        }
        let txn = db.transaction();

        // update tick
        let bs = txn.get(tick_key.as_bytes())?;
        let mut tick: Tick = serde_json::from_slice(&bs.unwrap()).unwrap();
        let amt = inp.amt.as_ref().unwrap().parse::<BigDecimal>().unwrap();
        let max = tick.max.as_ref().unwrap().parse::<BigDecimal>().unwrap();
        let minted = tick.minted.parse::<BigDecimal>().unwrap();
        let updated_minted = minted + amt;
        if updated_minted.gt(&max) {
            warn!("Max supply is reached, just ignore it!");
            return Ok(());
        }
        tick.minted = updated_minted.to_string();
        if updated_minted.eq(&max) {
            tick.end_block = Some(tx.block_number.unwrap().as_u64());
        }
        let tick_value = serde_json::to_string(&tick).unwrap();
        txn.put(tick_key.as_bytes(), tick_value.as_bytes())?;

        // insert mint
        let insc = DBInscription {
            id,
            chain_id,
            block: blockno,
            p: inp.p.to_owned(),
            op: inp.op.to_owned(),
            tick: inp.tick.to_owned(),
            max: inp.max.to_owned(),
            lim: inp.lim.to_owned(),
            amt: inp.amt.to_owned(),
            owner: tx.from.encode_hex(),
        };
        let insc_key = self.key_tick_minted(
            &inp.p,
            &inp.tick,
            &tx.hash.encode_hex(),
            block.timestamp.as_u64(),
        );
        let insc_value = serde_json::to_string(&insc).unwrap();
        txn.put(insc_key.as_bytes(), insc_value.as_bytes())?;

        // index block & txi
        self.persist_block(&txn, blockno, tx.transaction_index.unwrap().as_u64() as i64)?;
        txn.commit()?;
        Ok(())
    }

    fn persist_block(
        &self,
        txn: &rocksdb::Transaction<TransactionDB>,
        indexed_block: u64,
        indexed_txi: i64,
    ) -> Result<(), anyhow::Error> {
        let indexed_record = IndexedRecord {
            chain_id: self.chain_id,
            indexed_block,
            indexed_txi,
        };
        let indexed_key = self.key_indexed_record();
        let indexed_value = serde_json::to_string(&indexed_record).unwrap();
        txn.put(indexed_key.as_bytes(), indexed_value.as_bytes())?;
        Ok(())
    }
}
