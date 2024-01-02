use super::Indexer;

pub const WILDCARD: &'static str = "*";
pub trait Keys {
    fn key_indexed_record(&self) -> String;
    fn key_tick_mint(&self, p: &str, tick: &str, holder: &str, hash: &str, ts: u64) -> String;
    fn key_tick_deploy(&self, p: &str, tick: &str) -> String;
}

impl Keys for Indexer {
    fn key_indexed_record(&self) -> String {
        let p = if self.filter.p.is_some() {
            self.filter.p.as_deref().unwrap()
        } else {
            WILDCARD
        };
        let tick = if self.filter.tick.is_some() {
            self.filter.tick.as_deref().unwrap()
        } else {
            WILDCARD
        };
        format!("indexed#{}#{}#{}", self.chain_id, p, tick)
    }
    fn key_tick_mint(&self, p: &str, tick: &str, holder: &str, hash: &str, ts: u64) -> String {
        format!(
            "mint#{}#{}#{}#{}#{}#{}",
            self.chain_id, p, tick, holder, hash, ts
        )
    }
    fn key_tick_deploy(&self, p: &str, tick: &str) -> String {
        format!("deploy#{}#{}#{}", self.chain_id, p, tick)
    }
}
