use super::Tick;
use super::{IndexedRecord, Inscription};
use anyhow::anyhow;
use anyhow::Ok;
use bigdecimal::BigDecimal;
use ethers::{abi::AbiEncode, types::Transaction};
use log::warn;
use rocksdb::{Options, TransactionDB};
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn process_deploy(
    db: Arc<Mutex<TransactionDB>>,
    tx: &Transaction,
    inp: &Inscription,
) -> Result<Tick, anyhow::Error> {
    let mut db = db.lock().await;
    let chain_id = tx.chain_id.unwrap().as_u64();
    let start_block = tx.block_number.unwrap().as_u64();
    let id: String = tx.hash.encode_hex();
    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.create_missing_column_families(true);
    let cf_name = format!("tick#{}#{}#{}", chain_id, inp.p, inp.tick);
    let cf = db.cf_handle(&cf_name);

    if let Some(_) = cf {
        warn!("The tick has been deployed, just skip it!");
        let bs = db.get(cf_name.as_bytes())?;
        let tick: Tick = serde_json::from_slice(&bs.unwrap()).unwrap();
        return Ok(tick);
    }
    db.create_cf(cf_name.as_str(), &opts)?;
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
    let tick_key = &cf_name;
    let tick_value = serde_json::to_string(&tick).unwrap();
    txn.put(tick_key.as_bytes(), tick_value.as_bytes())?;

    // index block
    let indexed_record = IndexedRecord {
        chain_id,
        indexed_block: start_block,
        indexed_txi: tx.transaction_index.unwrap().as_u64() as i64,
    };
    let indexed_key = format!("indexed#{}", chain_id);
    let indexed_value = serde_json::to_string(&indexed_record).unwrap();
    txn.put(indexed_key.as_bytes(), indexed_value.as_bytes())?;

    txn.commit()?;
    Ok(tick)
}

pub async fn process_mint(
    db: Arc<Mutex<TransactionDB>>,
    tx: &Transaction,
    inp: &Inscription,
) -> Result<Tick, anyhow::Error> {
    let db = db.lock().await;
    let chain_id = tx.chain_id.unwrap().as_u64();
    let block = tx.block_number.unwrap().as_u64();
    let id: String = tx.hash.encode_hex();

    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.create_missing_column_families(true);

    let cf_name = format!("inscriptions#{}#{}#{}", chain_id, inp.p, inp.tick);
    let cf = db.cf_handle(cf_name.as_str());
    if let None = cf {
        return Err(anyhow!("Not found deployed tick"));
    }
    let cf = cf.unwrap();
    let txn = db.transaction();

    // update tick
    let bs = txn.get(cf_name.as_bytes())?;
    let mut tick: Tick = serde_json::from_slice(&bs.unwrap()).unwrap();
    let amt = inp.amt.as_ref().unwrap().parse::<BigDecimal>().unwrap();
    let max = tick.max.as_ref().unwrap().parse::<BigDecimal>().unwrap();
    let minted = tick.minted.parse::<BigDecimal>().unwrap();
    let updated_minted = minted + amt;
    if updated_minted.gt(&max) {
        return Err(anyhow!("Max supply is reached"));
    }
    tick.minted = updated_minted.to_string();
    if updated_minted.eq(&max) {
        tick.end_block = Some(tx.block_number.unwrap().as_u64());
    }
    let tick_key = format!("tick#{}#{}#{}", chain_id, inp.p, inp.tick);
    let tick_value = serde_json::to_string(&tick).unwrap();
    txn.put_cf(cf, tick_key.as_bytes(), tick_value.as_bytes())?;

    // insert mint
    let insc = Inscription {
        id,
        chain_id,
        block,
        p: inp.p.to_owned(),
        op: inp.op.to_owned(),
        tick: inp.tick.to_owned(),
        max: inp.max.to_owned(),
        lim: inp.lim.to_owned(),
        amt: inp.amt.to_owned(),
        owner: tx.from.encode_hex(),
    };
    let insc_key = format!("tick#{}#{}#{}", chain_id, inp.p, inp.tick);
    let insc_value = serde_json::to_string(&insc).unwrap();
    txn.put_cf(cf, insc_key.as_bytes(), insc_value.as_bytes())?;

    // index block & txi
    let indexed_record = IndexedRecord {
        chain_id,
        indexed_block: block,
        indexed_txi: tx.transaction_index.unwrap().as_u64() as i64,
    };
    let indexed_key = format!("indexed#{}", chain_id);
    let indexed_value = serde_json::to_string(&indexed_record).unwrap();
    txn.put(indexed_key.as_bytes(), indexed_value.as_bytes())?;

    txn.commit()?;
    Ok(tick)
}
