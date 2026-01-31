#!/bin/bash
# Demo script for agent-memory system
#
# This script demonstrates the full workflow:
# 1. Starts the daemon
# 2. Ingests sample conversation events
# 3. Shows storage statistics
# 4. Queries and displays results

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
DB_PATH="${PROJECT_DIR}/demo-data"
PORT="50051"
ENDPOINT="http://[::1]:${PORT}"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}╔══════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║           Agent Memory Demo                       ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════════════╝${NC}"
echo

# Change to project directory
cd "$PROJECT_DIR"

# Build the project
echo -e "${GREEN}[1/6] Building project...${NC}"
cargo build --release --quiet 2>/dev/null || cargo build --release
echo "      Build complete."
echo

# Clean previous demo data
echo -e "${GREEN}[2/6] Cleaning previous demo data...${NC}"
rm -rf "$DB_PATH"
echo "      Demo data directory: $DB_PATH"
echo

# Start the daemon in background
echo -e "${GREEN}[3/6] Starting daemon on port ${PORT}...${NC}"
./target/release/memory-daemon start --port "$PORT" --db-path "$DB_PATH" &
DAEMON_PID=$!
sleep 2

# Function to cleanup on exit
cleanup() {
    echo
    echo -e "${GREEN}[6/6] Stopping daemon...${NC}"
    kill $DAEMON_PID 2>/dev/null || true
    wait $DAEMON_PID 2>/dev/null || true
    echo "      Daemon stopped."
}
trap cleanup EXIT

# Check if daemon is running
if ! kill -0 $DAEMON_PID 2>/dev/null; then
    echo -e "${YELLOW}Warning: Daemon may have failed to start. Check logs.${NC}"
fi

echo "      Daemon started (PID: $DAEMON_PID)"
echo

# Ingest sample events via example
echo -e "${GREEN}[4/6] Ingesting sample conversation...${NC}"
MEMORY_ENDPOINT="$ENDPOINT" cargo run -p memory-daemon --release --example ingest_demo 2>/dev/null || \
    echo "      Note: Example failed. Ensure daemon is running."
echo

# Show stats
echo -e "${GREEN}[5/6] Storage statistics:${NC}"
./target/release/memory-daemon admin --db-path "$DB_PATH" stats
echo

# Query events from last hour
echo -e "${BLUE}═══════════════════════════════════════════════════${NC}"
echo -e "${BLUE}Query Examples${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════${NC}"
echo

echo -e "${YELLOW}Query TOC Root:${NC}"
./target/release/memory-daemon query --endpoint "$ENDPOINT" root 2>/dev/null || \
    echo "  (No TOC nodes yet - TOC building requires summarizer integration)"
echo

echo -e "${YELLOW}Query Recent Events:${NC}"
NOW=$(date +%s)000
HOUR_AGO=$(( $(date +%s) - 3600 ))000
./target/release/memory-daemon query --endpoint "$ENDPOINT" events \
    --from "$HOUR_AGO" --to "$NOW" --limit 5
echo

echo
echo -e "${BLUE}═══════════════════════════════════════════════════${NC}"
echo -e "${BLUE}Demo Complete${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════${NC}"
echo
echo "The daemon is still running. Press Ctrl+C to stop."
echo
echo "Try these commands manually:"
echo "  # Get storage stats"
echo "  ./target/release/memory-daemon admin --db-path $DB_PATH stats"
echo
echo "  # Query events"
echo "  ./target/release/memory-daemon query --endpoint $ENDPOINT events --from $HOUR_AGO --to $NOW"
echo
echo "  # Compact database"
echo "  ./target/release/memory-daemon admin --db-path $DB_PATH compact"
echo

# Wait for Ctrl+C
wait $DAEMON_PID
