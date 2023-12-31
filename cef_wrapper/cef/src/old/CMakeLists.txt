# Copyright (c) 2017 The Chromium Embedded Framework Authors. All rights
# reserved. Use of this source code is governed by a BSD-style license that
# can be found in the LICENSE file.

#
# Source files.
#

# Sources shared by all executables.
set(COMMON_SRCS
  app_factory.cc
  app_factory.h
  subprocess_util.cc
  subprocess_util.h
)

# Library sources.
set(LIB_SRCS
  ${COMMON_SRCS}
  app_browser.cc
  app_factory.cc
  app_factory.h
  client_display_handler.cc
  client_life_span_handler.cc
  client_manager.cc
  client_manager.h
  client_request_handler.cc
  client.h
  start_subprocess.cc
  start_subprocess.h
)

APPEND_PLATFORM_SOURCES(LIB_SRCS)

if(OS_MAC)
  # On macOS a separate helper executable is used for subprocesses.
  set(HELPER_SRCS
    ${COMMON_SRCS}
    process_helper_mac.cc
    process_helper_mac.h
  )
endif()


#
# Shared configuration.
#

# Target static library names.
set(TARGET "shared")
if(OS_MAC)
  set(HELPER_TARGET "helper")
endif()

if(OS_LINUX OR OS_WINDOWS)
  # Logical target used to link the libcef library on Linux and Windows.
  # On macOS the CEF framework is loaded dynamically at startup.
  ADD_LOGICAL_TARGET("libcef_lib" "${CEF_LIB_DEBUG}" "${CEF_LIB_RELEASE}")
endif()


#
# Linux configuration.
#

if(OS_LINUX)
  # Static library target.
  add_library(${SHARED_TARGET} STATIC ${SHARED_SRCS})
  SET_EXAMPLE_LIBRARY_TARGET_PROPERTIES(${SHARED_TARGET})
  add_dependencies(${SHARED_TARGET} libcef_dll_wrapper)
  target_link_libraries(${SHARED_TARGET} libcef_lib libcef_dll_wrapper ${CEF_STANDARD_LIBS})

  # Copy binary and resource files to the target output directory.
  COPY_FILES("${SHARED_TARGET}" "${CEF_BINARY_FILES}" "${CEF_BINARY_DIR}" "${EXAMPLE_TARGET_OUT_DIR}")
  COPY_FILES("${SHARED_TARGET}" "${CEF_RESOURCE_FILES}" "${CEF_RESOURCE_DIR}" "${EXAMPLE_TARGET_OUT_DIR}")

  # Output a message about setting SUID permissions on the chrome-sandbox target.
  SET_LINUX_SUID_PERMISSIONS("${SHARED_TARGET}" "${EXAMPLE_TARGET_OUT_DIR}/chrome-sandbox")
endif()


#
# Mac OS X configuration.
#

if(OS_MAC)
  # Helper static library target.
  add_library(${SHARED_HELPER_TARGET} STATIC ${SHARED_HELPER_SRCS})
  SET_EXAMPLE_LIBRARY_TARGET_PROPERTIES(${SHARED_HELPER_TARGET})
  add_dependencies(${SHARED_HELPER_TARGET} libcef_dll_wrapper)
  target_link_libraries(${SHARED_HELPER_TARGET} libcef_dll_wrapper ${CEF_STANDARD_LIBS})

  if(USE_SANDBOX)
    # Logical target used to link the cef_sandbox library.
    ADD_LOGICAL_TARGET("cef_sandbox_lib" "${CEF_SANDBOX_LIB_DEBUG}" "${CEF_SANDBOX_LIB_RELEASE}")
    target_link_libraries(${SHARED_HELPER_TARGET} cef_sandbox_lib)
  endif()

  # Main static library target.
  add_library(${SHARED_TARGET} STATIC ${SHARED_SRCS})
  SET_EXAMPLE_LIBRARY_TARGET_PROPERTIES(${SHARED_TARGET})
  add_dependencies(${SHARED_TARGET} libcef_dll_wrapper "${SHARED_HELPER_TARGET}")
  target_link_libraries(${SHARED_TARGET} libcef_dll_wrapper ${CEF_STANDARD_LIBS})
endif()


#
# Windows configuration.
#

if(OS_WINDOWS)
  # Create source groups for Visual Studio.
  SET_EXAMPLE_SOURCE_GROUPS("${SHARED_SRCS}")

  # Executable target.
  add_library(${SHARED_TARGET} STATIC ${SHARED_SRCS})
  SET_EXAMPLE_LIBRARY_TARGET_PROPERTIES(${SHARED_TARGET})
  add_dependencies(${SHARED_TARGET} libcef_dll_wrapper)
  target_link_libraries(${SHARED_TARGET} libcef_lib libcef_dll_wrapper ${CEF_STANDARD_LIBS})

  if(USE_SANDBOX)
    # Logical target used to link the cef_sandbox library.
    ADD_LOGICAL_TARGET("cef_sandbox_lib" "${CEF_SANDBOX_LIB_DEBUG}" "${CEF_SANDBOX_LIB_RELEASE}")
    target_link_libraries(${SHARED_TARGET} cef_sandbox_lib ${CEF_SANDBOX_STANDARD_LIBS})
  endif()

  # Copy binary and resource files to the target output directory.
  COPY_FILES("${SHARED_TARGET}" "${CEF_BINARY_FILES}" "${CEF_BINARY_DIR}" "${EXAMPLE_TARGET_OUT_DIR}")
  COPY_FILES("${SHARED_TARGET}" "${CEF_RESOURCE_FILES}" "${CEF_RESOURCE_DIR}" "${EXAMPLE_TARGET_OUT_DIR}")
endif()
