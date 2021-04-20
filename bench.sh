#!/bin/sh

# Runs a quick benchmark.
# ~/veloren currently has 23000 files in it.

set -e
cargo build --release
rm -f havit.sqlite
time target/release/havit add ~/veloren
ls -l havit.sqlite
rm havit.sqlite
time target/release/havit add ~/veloren
ls -l havit.sqlite

# no index, outside transaction: 11.7-11.9s, 3665920 bytes
# no index, inside transaction: 0.9-1.1s, 3665920 bytes
# with index, outside transaction: 13.4-13.6s, 5595136 bytes
# with index, inside transaction: 1.0-1.2s, 5595136 bytes

# Results:
# - Using a transaction cuts the time required by an order of magnitude.
# - Using an index increases the database size by 2X but doesn't add significantly to the time needed.

# Update

# Adding hashing made the run take 2X longer and the db 25% larger.
# Guessing computing the hash in parallel will pick up much of the dropped speed.

# no index, inside transaction, hashing: 1.9s, 4845568 bytes
