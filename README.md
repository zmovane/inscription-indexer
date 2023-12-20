# Inscription indexer on EVM

Since the start of the [brc-20 Experiment](https://domo-2.gitbook.io/brc-20-experiment/) initiated by domo, the Bitcoin ecosystem has begun to have a new narrative. Meanwhile, non-Bitcoin ecosystems have also followed the hype of Inscription.


### Inscription on EVM

Currently, there are three standard inscription formats. 

```
// Deploy
data:,{"p":"brc-20","op":"deploy","tick":"wakaka","max":"21000000","lim":"1000"}

// Mint
data:,{"p":"brc-20","op":"mint","tick":"wakaka","amt":"1000"}

// Transfer
data:,{"p":"brc-20","op":"transfer","tick":"wakaka","amt":"900"}
```

The data input is in hexadecimal string format and a self-transaction sent by a user is an inscription on EVM.


### How does inscription indexing work
Inscription need to be deployed first before users can start minting. Therefore, the indexer needs to be built from the first deploy, and the mint indexing ends when max supply is reached.

### DB migration

```
cargo run --bin prisma -- migrate dev
```

### Running indexer

```
RUST_LOG=info cargo run --bin inscription
```
