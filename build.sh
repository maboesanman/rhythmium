#!/bin/bash

cmake -B ./build -S . -G "Ninja Multi-Config"
# cmake --build build --target rust-cef-demo-bundle --config Release
cmake --build build --target rust-cef-demo-bundle --config Debug

