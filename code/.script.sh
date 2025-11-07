#!/bin/bash

rm -f kernel8.img
rm -f target/
cargo +nightly objcopy --target aarch64-unknown-none --bin code -- -O binary kernel8.img

