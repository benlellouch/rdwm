#!/bin/bash

set -e

killall -q polybar || true

polybar --config=/etc/polybar/config.ini example &

setxkbmap us

feh --bg-fill '/home/ben/gruvbox-wallpapers/wallpapers/photography/houseonthesideofalake.jpg' 

