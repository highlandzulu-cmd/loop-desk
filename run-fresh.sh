#!/bin/sh
# First-run experience: an empty config and no key in the environment, so the app asks you to connect a provider.
# Usage: ./run-fresh.sh [PROJECT_DIR]
here=$(cd "$(dirname "$0")" && pwd)
project=${1:-$PWD}
cd "$here" || exit 1
unset SOKET_API_KEY OPENROUTER_API_KEY OPENAI_API_KEY
export LOOP_CODING_AGENT_DIR=$HOME/.cache/loop-desk-fresh/agent
exec cargo run --release -- "$project"
