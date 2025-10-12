#!/bin/sh
cargo build

set -e 

xinit ./xinitrc -- $(command -v Xephyr) :2 -screen 1920x1080