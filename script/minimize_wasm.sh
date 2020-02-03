#!/bin/bash
cd target/web || exit 1

wasm-opt -O3 -o basegl_bg_opt.wasm basegl_bg.wasm
gzip --best --force basegl_bg_opt.wasm