# Copyright (c) 2016 The Chromium Embedded Framework Authors. All rights
# reserved. Use of this source code is governed by a BSD-style license that
# can be found in the LICENSE file.

# See the included README.md file for usage instructions.

# For VS2022 and Xcode 12+ support.
cmake_minimum_required(VERSION 3.21)

# Only generate Debug and Release configuration types.
set(CMAKE_CONFIGURATION_TYPES Debug Release)

# Project name.
project(cef)

# Use folders in the resulting project files.
set_property(GLOBAL PROPERTY OS_FOLDERS ON)


#
# CEF configuration.
#

# Determine the platform.
if("${CMAKE_SYSTEM_NAME}" STREQUAL "Darwin")
  if("${PROJECT_ARCH}" STREQUAL "arm64")
    set(CEF_PLATFORM "macosarm64")
  elseif("${PROJECT_ARCH}" STREQUAL "x86_64")
    set(CEF_PLATFORM "macosx64")
  elseif("${CMAKE_HOST_SYSTEM_PROCESSOR}" STREQUAL "arm64")
    set(PROJECT_ARCH "arm64")
    set(CEF_PLATFORM "macosarm64")
  else()
    set(PROJECT_ARCH "x86_64")
    set(CEF_PLATFORM "macosx64")
  endif()
elseif("${CMAKE_SYSTEM_NAME}" STREQUAL "Linux")
  if("${CMAKE_SYSTEM_PROCESSOR}" STREQUAL "arm")
    set(CEF_PLATFORM "linuxarm")
  elseif("${CMAKE_SYSTEM_PROCESSOR}" STREQUAL "arm64")
    set(CEF_PLATFORM "linuxarm64")
  elseif(CMAKE_SIZEOF_VOID_P MATCHES 8)
    set(CEF_PLATFORM "linux64")
  else()
    message(FATAL_ERROR "Linux x86 32-bit builds are discontinued.")
  endif()
elseif("${CMAKE_SYSTEM_NAME}" STREQUAL "Windows")
  if("${CMAKE_CXX_COMPILER_ARCHITECTURE_ID}" STREQUAL "ARM64")
    set(CEF_PLATFORM "windowsarm64")
  elseif(CMAKE_SIZEOF_VOID_P MATCHES 8)
    set(CEF_PLATFORM "windows64")
  else()
    set(CEF_PLATFORM "windows32")
  endif()
endif()

# Add this project's cmake/ directory to the module path.
set(CMAKE_MODULE_PATH ${CMAKE_MODULE_PATH} "${CMAKE_CURRENT_SOURCE_DIR}/cmake")

# Download and extract the CEF binary distribution (executes DownloadCEF.cmake).
include(CEFVersion)
include(DownloadCEF)
DownloadCEF("${CEF_PLATFORM}" "${CEF_VERSION}" "${CMAKE_SOURCE_DIR}/third_party/cef")

# Add the CEF binary distribution's cmake/ directory to the module path.
set(CMAKE_MODULE_PATH ${CMAKE_MODULE_PATH} "${CEF_ROOT}/cmake")

# Load the CEF configuration (executes FindCEF.cmake).
find_package(CEF REQUIRED)


#
# Python configuration.
#

# Support specification of the Python executable path via the command-line.
if(DEFINED ENV{PYTHON_EXECUTABLE})
  file(TO_CMAKE_PATH "$ENV{PYTHON_EXECUTABLE}" PYTHON_EXECUTABLE)
endif()

if(NOT PYTHON_EXECUTABLE)
  unset(PYTHON_EXECUTABLE)

  # Find the python interpreter.
  find_package(PythonInterp)

  if(NOT ${PYTHONINTERP_FOUND})
    message(FATAL_ERROR "A Python installation is required. Set the "
                        "PYTHON_EXECUTABLE environment variable to explicitly "
                        "specify the Python executable path.")
  endif()
endif()

message(STATUS "Using Python: ${PYTHON_EXECUTABLE}")


#
# Target configuration.
#

# Include the libcef_dll_wrapper target (executes libcef_dll/CMakeLists.txt).
add_subdirectory(${CEF_LIBCEF_DLL_WRAPPER_PATH} libcef_dll_wrapper)

# Include CEF's test application targets (executes <target>/CMakeLists.txt).
# add_subdirectory(${CEF_ROOT}/tests/cefclient)
# add_subdirectory(${CEF_ROOT}/tests/cefsimple)
# add_subdirectory(${CEF_ROOT}/tests/gtest)
# add_subdirectory(${CEF_ROOT}/tests/ceftests)

# Allow includes relative to the current source directory.
include_directories(${CMAKE_CURRENT_SOURCE_DIR})

# Configure building of the examples.
if(OS_MAC)
  # Shared macOS resources (menus, icons, etc).
  set(SHARED_RESOURCES_SRCS
    ../resources/mac/English.lproj/InfoPlist.strings
    ../resources/mac/English.lproj/MainMenu.xib
    ../resources/mac/Info.plist
    ../resources/mac/shared.icns
    )

  # Info.plist files used for main and helper app bundles.
  set(SHARED_INFO_PLIST ../resources/mac/Info.plist)
  set(SHARED_HELPER_INFO_PLIST ../resources/mac/helper-Info.plist)
elseif(OS_WINDOWS)
  # Shared Windows resources (version information, icons).
  set(SHARED_RESOURCES_SRCS
    ../resources/win/big.ico
    ../resources/win/resource.h
    ../resources/win/shared.rc
    ../resources/win/small.ico
    )

  # Manifest files used for executables.
  set(SHARED_EXE_MANIFEST ../resources/win/shared.exe.manifest)
  set(SHARED_COMPATIBILITY_MANIFEST ../resources/win/compatibility.manifest)
endif()

set(EXAMPLE_TARGET_OUT_DIR "${CMAKE_BINARY_DIR}/target_out")

# Set properties common to all example targets.
macro(SET_EXAMPLE_PROPERTIES target)
  # Output all binaries to the configuration-specific build directory.
  set_target_properties(${target} PROPERTIES
                        ARCHIVE_OUTPUT_DIRECTORY "${EXAMPLE_TARGET_OUT_DIR}"
                        RUNTIME_OUTPUT_DIRECTORY "${EXAMPLE_TARGET_OUT_DIR}"
                        LIBRARY_OUTPUT_DIRECTORY "${EXAMPLE_TARGET_OUT_DIR}")
endmacro()

macro(SET_EXAMPLE_LIBRARY_TARGET_PROPERTIES target)
  SET_LIBRARY_TARGET_PROPERTIES(${target})
  SET_EXAMPLE_PROPERTIES(${target})
endmacro()

macro(SET_EXAMPLE_EXECUTABLE_TARGET_PROPERTIES target)
  SET_EXECUTABLE_TARGET_PROPERTIES(${target})
  SET_EXAMPLE_PROPERTIES(${target})

  if(OS_LINUX)
    # Set rpath so that libraries can be placed next to the executable.
    set_target_properties(${target} PROPERTIES INSTALL_RPATH "$ORIGIN")
    set_target_properties(${target} PROPERTIES BUILD_WITH_INSTALL_RPATH TRUE)
  elseif(OS_WINDOWS)
    # Add the custom Windows manifest files to the executable.
    add_custom_command(
      TARGET ${target}
      POST_BUILD
      COMMAND "mt.exe" -nologo
              -manifest \"${CMAKE_CURRENT_SOURCE_DIR}/${SHARED_EXE_MANIFEST}\"
                        \"${CMAKE_CURRENT_SOURCE_DIR}/${SHARED_COMPATIBILITY_MANIFEST}\"
              -outputresource:"${EXAMPLE_TARGET_OUT_DIR}/${target}.exe"\;\#1
      COMMENT "Adding manifest..."
      )
  endif()
endmacro()

add_subdirectory(shared)
add_subdirectory(rhythmium)
