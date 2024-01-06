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
  client_render_handler.cc
  client.h
  create_browser.cc
  create_browser.h
  start_subprocess.cc
  start_subprocess.h
)

set(LIB_SRCS_LINUX
)

set(LIB_SRCS_POSIX
)

set(LIB_SRCS_WINDOWS
)

set(LIB_SRCS_MAC
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
# main static library target.
#

set(TARGET "cef_wrapper")

add_library(${TARGET} STATIC)

# a hack to use cef wrapper to convert c++ api types to c api types
target_compile_definitions(${TARGET} PUBLIC -DWRAPPING_CEF_SHARED)

target_sources(${TARGET} PRIVATE ${LIB_SRCS})
SET_LIBRARY_TARGET_PROPERTIES(${TARGET})
add_dependencies(${TARGET} libcef_dll_wrapper)
target_link_libraries(${TARGET} libcef_dll_wrapper ${CEF_STANDARD_LIBS} ${CEF_SANDBOX_STANDARD_LIBS} )

if(OS_LINUX OR OS_WINDOWS)
  # Logical target used to link the libcef library on Linux and Windows.
  # On macOS the CEF framework is loaded dynamically at startup.
  ADD_LOGICAL_TARGET("libcef_lib" "${CEF_LIB_DEBUG}" "${CEF_LIB_RELEASE}")
  target_link_libraries(${TARGET} libcef_lib ${CEF_STANDARD_LIBS} ${CEF_SANDBOX_STANDARD_LIBS})
endif()

if(OS_MAC OR OS_WINDOWS)
  if(USE_SANDBOX)
    # Logical target used to link the cef_sandbox library.
    ADD_LOGICAL_TARGET("cef_sandbox_lib" "${CEF_SANDBOX_LIB_DEBUG}" "${CEF_SANDBOX_LIB_RELEASE}")
    target_link_libraries(${TARGET} cef_sandbox_lib ${CEF_SANDBOX_STANDARD_LIBS})

    # if cmake build type is debug
    if(CMAKE_BUILD_TYPE MATCHES Debug)
      # Copy the debug build of the cef_sandbox library.
      COPY_SINGLE_FILE("${TARGET}" "${CEF_SANDBOX_LIB_DEBUG}" "${CMAKE_LIBRARY_OUTPUT_DIRECTORY}/libcef_sandbox.a")
    else()
      # Copy the release build of the cef_sandbox library.
      COPY_SINGLE_FILE("${TARGET}" "${CEF_SANDBOX_LIB_RELEASE}" "${CMAKE_LIBRARY_OUTPUT_DIRECTORY}/libcef_sandbox.a")
    endif()
  endif()
endif()

if(OS_LINUX OR OS_WINDOWS)
  # Copy binary and resource files to the target output directory.
  COPY_FILES("${TARGET}" "${CEF_BINARY_FILES}" "${CEF_BINARY_DIR}" "${TARGET_OUT_DIR}")
  COPY_FILES("${TARGET}" "${CEF_RESOURCE_FILES}" "${CEF_RESOURCE_DIR}" "${TARGET_OUT_DIR}")
endif()

if(OS_LINUX)
  # Output a message about setting SUID permissions on the chrome-sandbox target.
  SET_LINUX_SUID_PERMISSIONS("${SHARED_TARGET}" "${TARGET_OUT_DIR}/chrome-sandbox")
endif()


#
# mac framework bundle target.
#

if(OS_MAC)
  set(BINARY_NAME "rhythmium")
  # set(BINARY_NAME $ENV{CARGO_BIN_NAME})
  set(HELPER_TARGET "${BINARY_NAME}_helper")
  set(HELPER_OUTPUT_NAME "${BINARY_NAME} Helper")

  # Output path for the main app bundle.
  set(APP "${TARGET_OUT_DIR}/${BINARY_NAME}.app")

  # Variables referenced from the main Info.plist file.
  set(EXECUTABLE_NAME "${BINARY_NAME}")
  set(PRODUCT_NAME "${BINARY_NAME}")

  # main info.plist. this will likely be replaced by cpack eventually.
  set(_main_info_plist "${CMAKE_CURRENT_BINARY_DIR}/Info.plist")
  file(READ "${CMAKE_CURRENT_SOURCE_DIR}/${INFO_PLIST}" _plist_contents)
  string(REPLACE "\${EXECUTABLE_NAME}" "${EXECUTABLE_NAME}" _plist_contents ${_plist_contents})
  string(REPLACE "\${PRODUCT_NAME}" "${PRODUCT_NAME}" _plist_contents ${_plist_contents})
  file(WRITE ${_main_info_plist} ${_plist_contents})

  # Copy the Info.plist into the bundle.
  add_custom_command(
    TARGET ${TARGET}
    POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy
            "${_main_info_plist}"
            "${APP}/Contents/Info.plist"
    VERBATIM
  )

  # Copy the CEF framework into the Frameworks directory.
  add_custom_command(
    TARGET ${TARGET}
    POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_directory
            "${CEF_BINARY_DIR}/Chromium Embedded Framework.framework"
            "${APP}/Contents/Frameworks/Chromium Embedded Framework.framework"
    VERBATIM
  )

  add_custom_command(
    TARGET ${TARGET}
    POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_directory
            "${CEF_INCLUDE_PATH}/include"
            "${CMAKE_INCLUDE_OUTPUT_DIRECTORY}/include"
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

    # Create Helper executable target.
    add_executable(${_helper_target} MACOSX_BUNDLE ${HELPER_SRCS})
    SET_EXECUTABLE_TARGET_PROPERTIES(${_helper_target})
    add_dependencies(${_helper_target} libcef_dll_wrapper)
    target_link_libraries(${_helper_target} libcef_dll_wrapper ${CEF_STANDARD_LIBS})
    set_target_properties(${_helper_target} PROPERTIES
      MACOSX_BUNDLE_INFO_PLIST ${_helper_info_plist}
      OUTPUT_NAME ${_helper_output_name}
    )

    if(USE_SANDBOX)
      add_dependencies(${_helper_target} cef_sandbox_lib)
      target_link_libraries(${_helper_target} cef_sandbox_lib ${CEF_SANDBOX_STANDARD_LIBS})
    endif()

    # Add the Helper as a dependency of the main executable target.
    add_dependencies(${TARGET} "${_helper_target}")

    # Copy the Helper app bundle into the Frameworks directory.
    add_custom_command(
      TARGET ${TARGET}
      POST_BUILD
      COMMAND ${CMAKE_COMMAND} -E copy_directory
              "${CMAKE_RUNTIME_OUTPUT_DIRECTORY}/${_helper_output_name}.app"
              "${APP}/Contents/Frameworks/${_helper_output_name}.app"
      VERBATIM
    )
  endforeach()
endif()
