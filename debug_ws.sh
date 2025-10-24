#!/bin/bash

# Script to monitor WebSocket debug logs in real-time

echo "=== Kraken WebSocket Debug Monitor ==="
echo ""
echo "This script will monitor WebSocket activity in separate terminals."
echo "Press Ctrl+C to stop monitoring."
echo ""

# Clean up old logs
rm -f /tmp/kraken_ws_*.txt

echo "Starting monitors..."
echo ""

# Monitor raw WebSocket messages
echo "Terminal 1: Raw WebSocket Messages"
tail -f /tmp/kraken_ws_messages.txt 2>/dev/null &
PID1=$!

# Monitor parsed ticker updates
echo "Terminal 2: Ticker Updates"
tail -f /tmp/kraken_ws_tickers.txt 2>/dev/null &
PID2=$!

# Monitor redraw triggers
echo "Terminal 3: Redraw Triggers"
tail -f /tmp/kraken_ws_redraws.txt 2>/dev/null &
PID3=$!

# Monitor parse errors
echo "Terminal 4: Parse Errors"
tail -f /tmp/kraken_ws_parse_errors.txt 2>/dev/null &
PID4=$!

# Wait for Ctrl+C
trap "kill $PID1 $PID2 $PID3 $PID4 2>/dev/null; echo 'Stopped monitoring.'; exit" INT

echo ""
echo "Monitoring active. Press Ctrl+C to stop."
echo ""
echo "Quick summary:"
echo "  - Messages:  /tmp/kraken_ws_messages.txt"
echo "  - Tickers:   /tmp/kraken_ws_tickers.txt"
echo "  - Redraws:   /tmp/kraken_ws_redraws.txt"
echo "  - Errors:    /tmp/kraken_ws_parse_errors.txt"
echo ""

wait
