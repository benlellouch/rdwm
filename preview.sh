#!/bin/sh
cargo build

set -e 

xinit ./xinitrc -- $(command -v Xephyr) :2 -screen 2560x1440