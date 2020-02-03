#!/bin/bash

echoerr() { echo "$@" 1>&2; }

cd target/web || exit 1

current_size=$(du -h basegl_bg_opt.wasm.gz | awk '{ print $1 }')
current_size="${current_size::-1}"

max_size=2.1 # MB
echo "Current size: ${current_size}M. expected maximum size: ${max_size}M"
if (( $(echo "$current_size <= $max_size" |bc -l) ));
then
  echo OK
else
  echoerr FAIL
  exit 1
fi