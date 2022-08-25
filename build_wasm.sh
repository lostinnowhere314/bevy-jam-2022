
## Builds the WASM build and packages everything together

#abort on errors
set -e
cargo check --all-targets
# Compile the wasm release profile
cargo build --profile wasm-release --target wasm-unknown-unknown

# Bindgen
wasm-bindgen --out-name lin_bevy_jam --out-dir wasm --target web target/wasm32-unknown-unknown/release/lin_bevy_jam.wasm

bash package_wasm.sh