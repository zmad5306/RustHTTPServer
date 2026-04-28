# Prime

## Build & Run on Linux

```/bin/bash
curl https://sh.rustup.rs -sSf | sh
source $HOME/.cargo/env
rustc --version
cargo --version
rustup update
cd prime
cargo build --release
chmod +x ./target/release/RustPrime
./target/release/RustPrime
```
