#!/bin/bash
set -e

echo "🚀 Building minimal production container image..."
nix build .#container-tarball

echo "📦 Copying to nixos-production.tar.gz..."
cp result nixos-production.tar.gz

echo "📊 Container image stats:"
ls -lh nixos-production.tar.gz

echo "✅ Production container ready!"
echo "   Use: ./target/debug/cli create --image-path ./nixos-production.tar.gz ..." 