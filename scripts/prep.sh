#!/bin/bash

cmake -B ./build -S . -G "Ninja" -DCMAKE_BUILD_TYPE=Release && cmake --build build --target rhythmium_partial_bundle

mkdir cache_root