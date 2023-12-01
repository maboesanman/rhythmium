#!/bin/bash

mkdir build

cd build

cmake -G "Ninja" -DPROJECT_ARCH="arm64" ..

ninja rust-cef-demo
ninja mac_helper_process
