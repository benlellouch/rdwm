#!/bin/sh

LOGDIR="$HOME/.logs"
LOGFILE="$LOGDIR/rdwm.log"

mkdir -p "$LOGDIR"

export RUST_LOG=debug

echo "=== starting rdwm supervisor: $(date) ===" >> "$LOGFILE"

while :; do
  echo "--- rdwm launch: $(date) ---" >> "$LOGFILE"

  # Run rdwm; capture stdout+stderr; line-buffer for faster logs
  stdbuf -oL -eL /home/ben/Projects/rdwm/target/release/rdwm >> "$LOGFILE" 2>&1

  status=$?
  echo "--- rdwm exited (status=$status): $(date) ---" >> "$LOGFILE"

  # If it exited cleanly (e.g. you implement "exit to logout"), stop restarting
  [ "$status" -eq 0 ] && break

  # Avoid a tight restart loop if it crashes instantly
  sleep 1
done
