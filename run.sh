#!/bin/sh
# Open the desktop app. Usage: ./run.sh [PROJECT_DIR]   (default: the current folder)
# The first launch asks for an API key in the app. A SOKET_API_KEY (or OPENROUTER_API_KEY / OPENAI_API_KEY)
# in the environment or in a .env file next to this script is used instead. LOOP_CODING_AGENT_DIR picks
# another Loop config folder.
here=$(cd "$(dirname "$0")" && pwd)
project=${1:-$PWD}
cd "$here" || exit 1
if [ -f .env ]; then set -a; . ./.env; set +a; fi
exec cargo run --release -- "$project"
