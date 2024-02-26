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
set(CMAKE_MODULE_PATH ${CMAKE_MODULE_PATH} "${CMAKE_CURRENT_SOURCE_DIR}/cmake")

# Download and extract the CEF binary distribution (executes DownloadCEF.cmake).
include(DownloadCEF)
DownloadCEF("${CMAKE_SOURCE_DIR}/third_party/cef")

# Add the CEF binary distribution's cmake/ directory to the module path.
set(CMAKE_MODULE_PATH ${CMAKE_MODULE_PATH} "${CEF_ROOT}/cmake")

# Load the CEF configuration (executes FindCEF.cmake).
find_package(CEF REQUIRED)

# Set up corrosion

include(FetchContent)

FetchContent_Declare(
  Corrosion
  GIT_REPOSITORY https://github.com/corrosion-rs/corrosion.git
  GIT_TAG v0.4 # Optionally specify a commit hash, version tag or branch here
)
# Set any global configuration variables such as `Rust_TOOLCHAIN` before this line!
FetchContent_MakeAvailable(Corrosion)

corrosion_import_crate(MANIFEST_PATH ${CMAKE_SOURCE_DIR}/Cargo.toml)

if(OS_MAC)
  set(INFO_PLIST ./resources/mac/Info.plist)
  set(HELPER_INFO_PLIST ./resources/mac/helper-Info.plist)

  add_custom_target(macos_framework_dir)
  add_dependencies(macos_framework_dir cef_mac_helper)

  set(BINARY_NAME "rhythmium")
  # set(BINARY_NAME $ENV{CARGO_BIN_NAME})
  set(HELPER_TARGET "${BINARY_NAME}_helper")
  set(HELPER_OUTPUT_NAME "${BINARY_NAME} Helper")

  # Output path for the main app bundle.
  set(APP "${CMAKE_RUNTIME_OUTPUT_DIRECTORY}/${BINARY_NAME}.app")
  set(PARTIAL_BUNDLE "${CMAKE_LIBRARY_OUTPUT_DIRECTORY}/${BINARY_NAME}_partial_bundle.app")

  # Variables referenced from the main Info.plist file.
  set(EXECUTABLE_NAME "${BINARY_NAME}")
  set(PRODUCT_NAME "${BINARY_NAME}")

  # main info.plist. this will likely be replaced by cpack eventually.
  set(_main_info_plist "${CMAKE_CURRENT_BINARY_DIR}/Info.plist")
  file(READ "${CMAKE_CURRENT_SOURCE_DIR}/${INFO_PLIST}" _plist_contents)
  string(REPLACE "\${EXECUTABLE_NAME}" "${EXECUTABLE_NAME}" _plist_contents ${_plist_contents})
  string(REPLACE "\${PRODUCT_NAME}" "${PRODUCT_NAME}" _plist_contents ${_plist_contents})
  file(WRITE ${_main_info_plist} ${_plist_contents})

  add_custom_target(${BINARY_NAME}_partial_bundle ALL
    COMMAND ${CMAKE_COMMAND} -E make_directory
            "${PARTIAL_BUNDLE}/Contents/MacOS"
    COMMAND ${CMAKE_COMMAND} -E copy
            "${_main_info_plist}"
            "${PARTIAL_BUNDLE}/Contents/Info.plist"
    COMMAND ${CMAKE_COMMAND} -E copy_directory
            "${CMAKE_LIBRARY_OUTPUT_DIRECTORY}/Frameworks"
            "${PARTIAL_BUNDLE}/Contents/Frameworks"
    VERBATIM)

  add_dependencies(${BINARY_NAME}_partial_bundle macos_framework_dir)

  add_custom_target(${BINARY_NAME}_app ALL
    COMMAND ${CMAKE_COMMAND} -E copy_directory
            "${PARTIAL_BUNDLE}"
            "${APP}"
    COMMAND ${CMAKE_COMMAND} -E copy
            "${CMAKE_RUNTIME_OUTPUT_DIRECTORY}/${BINARY_NAME}"
            "${APP}/Contents/MacOS/${BINARY_NAME}"
    VERBATIM)

  add_dependencies(${BINARY_NAME}_app ${BINARY_NAME} ${BINARY_NAME}_partial_bundle)

  set(FRAMEWORKS_DIR "${CMAKE_LIBRARY_OUTPUT_DIRECTORY}/Frameworks")

  # Create the Frameworks directory.
  add_custom_command(
    TARGET macos_framework_dir
    POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E make_directory
            "${FRAMEWORKS_DIR}"
    VERBATIM
  )

  # Copy the CEF framework into the Frameworks directory.
  add_custom_command(
    TARGET macos_framework_dir
    POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_directory
            "${CEF_BINARY_DIR}/Chromium Embedded Framework.framework"
            "${FRAMEWORKS_DIR}/Chromium Embedded Framework.framework"
    VERBATIM
  )

  # Create the multiple Helper app bundle targets.
  foreach(_suffix_list ${CEF_HELPER_APP_SUFFIXES})
    # Convert to a list and extract the suffix values.
    string(REPLACE ":" ";" _suffix_list ${_suffix_list})
    list(GET _suffix_list 0 _name_suffix)
    list(GET _suffix_list 1 _target_suffix)
    list(GET _suffix_list 2 _plist_suffix)

    # Define Helper target and output names.
    set(_helper_target "${HELPER_TARGET}${_target_suffix}")
    set(_helper_output_name "${HELPER_OUTPUT_NAME}${_name_suffix}")

    # Create Helper-specific variants of the helper-Info.plist file. Do this
    # manually because the configure_file command (which is executed as part of
    # MACOSX_BUNDLE_INFO_PLIST) uses global env variables and would insert the
    # wrong values with multiple targets.
    set(_helper_info_plist "${CMAKE_CURRENT_BINARY_DIR}/${BINARY_NAME}-helper-Info${_target_suffix}.plist")
    file(READ "${CMAKE_CURRENT_SOURCE_DIR}/${HELPER_INFO_PLIST}" _plist_contents)
    string(REPLACE "\${EXECUTABLE_NAME}" "${_helper_output_name}" _plist_contents ${_plist_contents})
    string(REPLACE "\${PRODUCT_NAME}" "${_helper_output_name}" _plist_contents ${_plist_contents})
    string(REPLACE "\${BUNDLE_ID_SUFFIX}" "${_plist_suffix}" _plist_contents ${_plist_contents})
    file(WRITE ${_helper_info_plist} ${_plist_contents})

    add_custom_target(${_helper_target} ALL
      COMMAND ${CMAKE_COMMAND} -E make_directory
              "${CMAKE_LIBRARY_OUTPUT_DIRECTORY}/${_helper_output_name}.app"
      COMMAND ${CMAKE_COMMAND} -E copy
              "${CMAKE_RUNTIME_OUTPUT_DIRECTORY}/cef_mac_helper"
              "${CMAKE_LIBRARY_OUTPUT_DIRECTORY}/${_helper_output_name}.app/Contents/MacOS/${_helper_output_name}"
      COMMAND ${CMAKE_COMMAND} -E copy
              "${_helper_info_plist}"
              "${CMAKE_LIBRARY_OUTPUT_DIRECTORY}/${_helper_output_name}.app/Contents/Info.plist"
      VERBATIM
    )
    
    add_dependencies(${_helper_target} cef_mac_helper)
    add_dependencies(macos_framework_dir ${_helper_target})

    # Copy the Helper app bundle into the Frameworks directory.
    add_custom_command(
      TARGET macos_framework_dir
      POST_BUILD
      COMMAND ${CMAKE_COMMAND} -E copy_directory
              "${CMAKE_LIBRARY_OUTPUT_DIRECTORY}/${_helper_output_name}.app"
              "${FRAMEWORKS_DIR}/${_helper_output_name}.app"
      VERBATIM
    )
  endforeach()
endif()