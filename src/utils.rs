use ethers::{
    types::{H160, H256},
    utils::hex::ToHex,
};
use regex::Regex;
use std::str::FromStr;

pub fn remove_leadering_zeros(hex: String) -> String {
    let pattern: Regex = Regex::new("^0x0{24}").unwrap();
    pattern.replace(hex.as_str(), "0x").to_string()
}

pub fn h256_to_h160(h256: H256) -> H160 {
    let h256_hex = h256.encode_hex();
    let h160_hex = remove_leadering_zeros(h256_hex);
    H160::from_str(h160_hex.as_str()).unwrap()
}
