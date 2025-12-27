#!/bin/sh
WM_TESTING=True cargo build --release

set -e 

xinit ./xinitrc -- $(command -v Xephyr) :2 -screen 2560x1440