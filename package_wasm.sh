
## Updates assets and packages the WASM build (w/o compiling)
set -e
# Update assets
cp -rfu assets wasm/assets

cd wasm
zip -ru ../wasm-build.zip .
cd ..
