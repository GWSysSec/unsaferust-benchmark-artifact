#!/bin/bash

# Script to update repositories (submodules) before building Docker image

set -e

echo "========================================"
echo "Updating submodules to latest changes"
echo "========================================"
echo ""

# Initialize and update all submodules
echo "1. Updating git submodules..."
git submodule update --init --recursive

echo ""
echo "========================================"
echo "Repository update complete!"
echo "========================================"
echo ""
