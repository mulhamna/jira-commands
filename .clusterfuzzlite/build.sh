#!/bin/bash
set -euo pipefail

cd $SRC/jira-commands
cargo fuzz build -O markdown_adf
cp fuzz/target/x86_64-unknown-linux-gnu/release/markdown_adf $OUT/
