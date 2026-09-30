#!/usr/bin/env bash
# Builds the Rust engine for both wrappers : the Python extension and the browser module
set -euo pipefail

racine="$(cd "$(dirname "$0")" && pwd)"
cd "$racine/rust"

cargo test -p champ_core

python3 -m maturin build --release -m py/Cargo.toml
roue=$(ls -t "$racine"/rust/target/wheels/champ_py-*.whl | head -1)
python3 -m pip install --quiet --force-reinstall "$roue"

wasm-pack build wasm --release --target web --out-dir "$racine/web/pkg" --out-name champ

# wasm-pack drops a .gitignore holding a star, which would keep the module out of any Pages deploy
rm -f "$racine/web/pkg/.gitignore" "$racine/web/pkg/package.json"
# Pages runs Jekyll otherwise, and it drops the files whose name starts with an underscore
touch "$racine/web/.nojekyll"

echo "Construit : extension Python installée, site autonome dans web/"
