# Copyright (c) 2016 The Chromium Embedded Framework Authors. All rights
# reserved. Use of this source code is governed by a BSD-style license that
# can be found in the LICENSE file.

# See the included README.md file for usage instructions.

# For VS2022 and Xcode 12+ support.
cmake_minimum_required(VERSION 3.21)

# Only generate Debug and Release configuration types.
set(CMAKE_CONFIGURATION_TYPES Debug Release)

# Project name.
project(cef_sys)

# Use folders in the resulting project files.
set_property(GLOBAL PROPERTY OS_FOLDERS ON)

set(CMAKE_ARCHIVE_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/lib)
set(CMAKE_LIBRARY_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/lib)
set(CMAKE_RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin)
set(CMAKE_INCLUDE_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/include)

# Add this project's cmake/ directory to the module path.
set(CMAKE_MODULE_PATH ${CMAKE_MODULE_PATH} "${CMAKE_CURRENT_SOURCE_DIR}/../cmake")

# Download and extract CEF.
include(DownloadCEF)
DownloadCEF("${CMAKE_SOURCE_DIR}/../third_party/cef")

# Add the CEF binary distribution's cmake/ directory to the module path.
set(CMAKE_MODULE_PATH ${CMAKE_MODULE_PATH} "${CEF_ROOT}/cmake")

# Load the CEF configuration (executes FindCEF.cmake).
find_package(CEF REQUIRED)

# Include the libcef_dll_wrapper target (executes libcef_dll/CMakeLists.txt).
add_subdirectory(${CEF_LIBCEF_DLL_WRAPPER_PATH} libcef_dll_wrapper)

# Copy the CEF include directory.
add_custom_target(copy_cef_include)

add_custom_command(
  TARGET copy_cef_include
  POST_BUILD
  COMMAND ${CMAKE_COMMAND} -E copy_directory
          "${CEF_INCLUDE_PATH}/include"
          "${CMAKE_INCLUDE_OUTPUT_DIRECTORY}/include"
  VERBATIM
)

# Copy the sandbox library.
add_custom_target(copy_cef_sandbox)

add_custom_command(
  TARGET copy_cef_include
  POST_BUILD
  COMMAND ${CMAKE_COMMAND} -E copy
          "${CEF_INCLUDE_PATH}/Release/cef_sandbox.a"
          "${CMAKE_ARCHIVE_OUTPUT_DIRECTORY}/libcef_sandbox.a"
  VERBATIM
)
