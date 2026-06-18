#!/usr/bin/env bash
# Build the single self-contained uuBot binary (frontend embedded).
set -euo pipefail

cd "$(dirname "$0")"

echo "==> Building frontend"
( cd frontend && npm install && npm run build )

echo "==> Building release binary (frontend embedded via rust-embed)"
# Touch embed.rs so rust-embed re-reads the freshly built frontend/dist.
touch src/embed.rs
cargo build --release

echo
echo "Done. Run it with:"
echo "  ./target/release/uuBot"
echo "Configure via environment variables or a .env file (see .env.example)."
