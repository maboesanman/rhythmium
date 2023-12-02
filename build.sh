#!/bin/bash

mkdir build

cd build

cmake -G "Ninja" -DPROJECT_ARCH="arm64" ..

ninja rust-cef-demo-bundle
# ninja helper_process-bundle
# ninja mac_helper_process

# cmake -G "Xcode" -DPROJECT_ARCH="arm64" ..
