use super::{
    database::Persistable, Indexer, Inscription, InscriptionFieldValidate, OP_DEPLOY, OP_MINT,
    PREFIX_INSCRIPTION,
};
use crate::config::Random;
use anyhow::{anyhow, Ok};
use ethers::{
    providers::{Middleware, StreamExt},
    types::{Block, BlockNumber, Transaction, H256},
};
use log::info;

impl Indexer {
    pub async fn index_inscriptions(&self) -> Result<(), anyhow::Error> {
        let (indexed_block, mut block_txi): (u64, i64) =
            self.get_indexed_block(self.indexed_type.to_owned()).await;
        let mut block_to_process = indexed_block;
        if self.filter.start_block.is_some() && self.filter.start_block.unwrap() != block_to_process
        {
            block_to_process = self.filter.start_block.unwrap();
        }
        let mut block_stream = self.wss.watch_blocks().await?;
        let next_block = move |block, txi| async move {
            let db = self.db.lock().await;
            let txn = db.transaction();
            self.persist_block(&txn, block, txi).unwrap();
            txn.commit().unwrap();
            (block + 1, -1)
        };
        while block_stream.next().await.is_some() {
            let block = self
                .https
                .random()
                .unwrap()
                .get_block(BlockNumber::Latest)
                .await?
                .unwrap();
            let block_number = block.number.unwrap();
            if block_to_process <= block_number.as_u64() {
                info!("Process block {}", block_to_process);
                let txs = self
                    .https
                    .random()
                    .unwrap()
                    .get_block_with_txs(block_to_process)
                    .await?;
                if let None = txs {
                    (block_to_process, block_txi) = next_block(block_to_process, block_txi).await;
                    continue;
                }
                let mut txs = txs.unwrap().transactions;
                txs = txs
                    .into_iter()
                    .filter(|tx| tx.transaction_index.unwrap().as_u64() as i64 > block_txi)
                    .collect::<Vec<Transaction>>();
                txs.sort_by(|x, y| x.transaction_index.cmp(&y.transaction_index));
                for tx in txs.iter() {
                    let (found, txi) = self.process_transaction(&block, tx).await?;
                    if !found {
                        continue;
                    }
                    block_txi = txi.unwrap();
                }
            }
            (block_to_process, block_txi) = next_block(block_to_process, block_txi).await;
            if self.filter.end_block.is_some() && block_to_process > self.filter.end_block.unwrap()
            {
                info!(
                    "Indexing was ended of block {}",
                    self.filter.end_block.unwrap()
                );
                break;
            }
        }
        Ok(())
    }

    async fn process_transaction(
        &self,
        block: &Block<H256>,
        tx: &Transaction,
    ) -> Result<(bool, Option<i64>), anyhow::Error> {
        let tx_without_valid_inscription = (false, None);
        if tx.to.is_none() {
            return Ok(tx_without_valid_inscription);
        }
        if self.filter.is_self_transaction && tx.to.unwrap().ne(&tx.from) {
            return Ok(tx_without_valid_inscription);
        }
        if self.filter.recipient.is_some() && tx.to.unwrap().ne(&self.filter.recipient.unwrap()) {
            return Ok(tx_without_valid_inscription);
        }
        let input = String::from_utf8(tx.input.to_vec())?;
        if !input.starts_with(PREFIX_INSCRIPTION) {
            return Ok(tx_without_valid_inscription);
        }
        let data = input.strip_prefix(PREFIX_INSCRIPTION).unwrap_or("{}");
        let deserialized = serde_json::from_str::<serde_json::Value>(data);
        if deserialized.is_err() {
            return Ok(tx_without_valid_inscription);
        }
        let deserialized = deserialized.unwrap();
        if !deserialized.is_object() {
            return Ok(tx_without_valid_inscription);
        }
        if !deserialized.is_valid_inscription() {
            return Ok(tx_without_valid_inscription);
        }
        let inscription: Inscription = serde_json::from_value(deserialized)?;
        if self.filter.p.is_some() && self.filter.p.as_ref().unwrap().ne(&inscription.p) {
            return Ok(tx_without_valid_inscription);
        }
        if self.filter.tick.is_some() && self.filter.tick.as_ref().unwrap().ne(&inscription.tick) {
            return Ok(tx_without_valid_inscription);
        }
        let (_, indexed_txi) = self.process_inscription(block, tx, &inscription).await?;
        Ok((true, Some(indexed_txi)))
    }

    async fn process_inscription(
        &self,
        block: &Block<H256>,
        tx: &Transaction,
        inp: &Inscription,
    ) -> Result<(u64, i64), anyhow::Error> {
        let op = inp.op.as_str();
        let _ = match op {
            OP_MINT => self.persist_mint(block, tx, inp).await?,
            OP_DEPLOY => self.persist_deploy(block, tx, inp).await?,
            _ => return Err(anyhow!("Invalid operations")),
        };
        let indexed_block = tx.block_number.unwrap().as_u64() as u64;
        let indexed_txi = tx.transaction_index.unwrap().as_u64() as i64;
        Ok((indexed_block, indexed_txi))
    }
}
