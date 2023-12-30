use super::Indexer;

pub const WILDCARD: &'static str = "*";
pub trait Keys {
    fn key_indexed_record(&self) -> String;
    fn key_tick_minted(&self, p: &str, tick: &str) -> String;
    fn key_tick_deployed(&self, p: &str, tick: &str) -> String;
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
    fn key_tick_minted(&self, p: &str, tick: &str) -> String {
        format!("inscriptions#{}#{}#{}", self.chain_id, p, tick)
    }
    fn key_tick_deployed(&self, p: &str, tick: &str) -> String {
        format!("tick#{}#{}#{}", self.chain_id, p, tick)
    }
}
