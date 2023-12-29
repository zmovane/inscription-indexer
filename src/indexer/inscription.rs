use super::{
    database::{process_deploy, process_mint},
    Indexer, Inscription, InscriptionFieldValidate, OP_DEPLOY, OP_MINT, PREFIX_INSCRIPTION,
    PREFIX_INSCRIPTION_HEX,
};
use crate::config::Random;
use anyhow::{anyhow, Ok};
use ethers::{
    abi::AbiEncode,
    providers::{Middleware, StreamExt},
    types::{BlockNumber, Transaction},
};

impl Indexer {
    pub async fn index_inscriptions(&self) -> Result<(), anyhow::Error> {
        let (indexed_block, mut block_txi): (u64, i64) =
            self.get_indexed_block(self.indexed_type.to_owned()).await;
        let mut block_to_process = indexed_block as u64;
        let mut block_stream = self.wss.watch_blocks().await?;
        let next_block = |block, _| -> (u64, i64) { (block + 1, -1) };
        while block_stream.next().await.is_some() {
            let block_number = self
                .https
                .random()
                .unwrap()
                .get_block(BlockNumber::Latest)
                .await?
                .unwrap()
                .number
                .unwrap();
            if block_to_process <= block_number.as_u64() {
                let txs = self
                    .https
                    .random()
                    .unwrap()
                    .get_block_with_txs(block_to_process)
                    .await?;
                if let None = txs {
                    (block_to_process, block_txi) = next_block(block_to_process, block_txi);
                    continue;
                }
                let mut txs = txs.unwrap().transactions;
                txs = txs
                    .into_iter()
                    .filter(|tx| tx.transaction_index.unwrap().as_u64() as i64 > block_txi)
                    .collect::<Vec<Transaction>>();
                txs.sort_by(|x, y| x.transaction_index.cmp(&y.transaction_index));
                for tx in txs.iter() {
                    let (found, txi) = self.process_transaction(tx).await?;
                    if !found {
                        continue;
                    }
                    block_txi = txi.unwrap();
                }
            }
            (block_to_process, block_txi) = next_block(block_to_process, block_txi);
        }
        Ok(())
    }

    async fn process_transaction(
        &self,
        tx: &Transaction,
    ) -> Result<(bool, Option<i64>), anyhow::Error> {
        let tx_without_inscription = (false, None);
        if tx.to.is_none() {
            let err = anyhow!("Invalid transaction {}: to is None", tx.hash);
            return Err(err);
        }
        if tx.to.unwrap().ne(&tx.from) {
            return Ok(tx_without_inscription);
        }
        if !tx
            .input
            .to_owned()
            .encode_hex()
            .starts_with(PREFIX_INSCRIPTION_HEX)
        {
            return Ok(tx_without_inscription);
        }
        let input = String::from_utf8(tx.input.to_vec())?;
        let data = input.strip_prefix(PREFIX_INSCRIPTION).unwrap_or("{}");
        let deserialized = serde_json::from_str::<serde_json::Value>(data);
        if deserialized.is_err() {
            return Ok(tx_without_inscription);
        }
        let deserialized = deserialized.unwrap();
        if !deserialized.is_object() {
            return Ok(tx_without_inscription);
        }
        if !deserialized.is_valid_inscription() {
            return Ok(tx_without_inscription);
        }
        let inscription: Inscription = serde_json::from_value(deserialized)?;
        let (_, indexed_txi) = self.process_inscription(tx, &inscription).await?;
        Ok((true, Some(indexed_txi)))
    }

    async fn process_inscription(
        &self,
        tx: &Transaction,
        inp: &Inscription,
    ) -> Result<(u64, i64), anyhow::Error> {
        let op = inp.op.as_str();
        let _ = match op {
            OP_MINT => process_mint(self.db.to_owned(), tx, inp).await?,
            OP_DEPLOY => process_deploy(self.db.to_owned(), tx, inp).await?,
            _ => return Err(anyhow!("Invalid operations")),
        };
        let indexed_block = tx.block_number.unwrap().as_u64() as u64;
        let indexed_txi = tx.transaction_index.unwrap().as_u64() as i64;
        Ok((indexed_block, indexed_txi))
    }
}
