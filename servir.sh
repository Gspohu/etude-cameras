#!/usr/bin/env bash
# Serves the page, the browser refuses WebAssembly and modules over file://
set -euo pipefail

port="${1:-8000}"
cd "$(cd "$(dirname "$0")" && pwd)"

echo "Page : http://localhost:$port/web/index.html"
python3 -m http.server "$port"
