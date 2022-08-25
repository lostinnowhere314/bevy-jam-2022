
# Update assets
cp -rfu assets wasm/assets

echo "Open https://localhost:8000/index.html to view"

python -m http.server --directory wasm