#!/bin/bash

# 增加文件描述符限制
ulimit -n 4096

# 运行 map viewer
cargo run --bin map_viewer_macroquad_v2 --features backend-macroquad --release
