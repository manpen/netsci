#!/usr/bin/env bash
ffmpeg -framerate 3 -pattern_type glob -i 'animation/*.png'   -c:v libx264 -pix_fmt yuv420p out.mp4