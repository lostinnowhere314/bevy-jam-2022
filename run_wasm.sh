wasm-bindgen --out-name lin_bevy_jam --out-dir wasm --target web target/wasm32-unknown-unknown/release/lin_bevy_jam.wasm

cp -rfu assets wasm/assets

echo "Open https://localhost:8000/index.html to view"

python -m http.server --directory wasm