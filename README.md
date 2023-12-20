# EVM Inscription indexer

### DB migration

```
cargo run --bin prisma -- migrate dev
```

### Running indexer

```
RUST_LOG=info cargo run --bin inscription
```
