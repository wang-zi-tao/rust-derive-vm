#!/usr/bin/env bash
script="local i, s, n = 1, 8, 1000000000 $1 while i < n do i = i + s $2 $3 $3 $3 $3 $3 $3 $3 $3 end"
RUST_LOG=0 ./target/debug/vm-lua-shell -b -e "$script"
time lua -e "$script"
