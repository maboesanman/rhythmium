#
# Shared resources. These are included by the individual executable targets so
# the paths must be relative to those directories.
#

if(OS_MAC)
  # Shared macOS resources (menus, icons, etc).
  set(RESOURCES_SRCS
    ./resources/mac/English.lproj/InfoPlist.strings
    ./resources/mac/English.lproj/MainMenu.xib
    ./resources/mac/Info.plist
    ./resources/mac/shared.icns
  )

  # Info.plist files used for main and helper app bundles.
  set(INFO_PLIST ./resources/mac/Info.plist)
  set(HELPER_INFO_PLIST ./resources/mac/helper-Info.plist)
elseif(OS_WINDOWS)
  # Shared Windows resources (version information, icons).
  set(RESOURCES_SRCS
    ./resources/win/big.ico
    ./resources/win/resource.h
    ./resources/win/shared.rc
    ./resources/win/small.ico
  )

  # Manifest files used for executables.
  set(EXE_MANIFEST ./resources/win/shared.exe.manifest)
  set(COMPATIBILITY_MANIFEST ./resources/win/compatibility.manifest)
endif()

# Set the configuration-specific binary output directory.
if(GEN_NINJA OR GEN_MAKEFILES)
  # Force Ninja and Make to create a subdirectory named after the configuration.
  set(TARGET_OUT_DIR "${CMAKE_BINARY_DIR}/${CMAKE_BUILD_TYPE}")
else()
  set(TARGET_OUT_DIR "${CMAKE_BINARY_DIR}/$<CONFIGURATION>")
endif()

set(TARGET "rust-cef-demo")

if(OS_MAC)
  set(HELPER_TARGET "helper_process")
  set(HELPER_OUTPUT_NAME "${TARGET} Helper")
endif()

if(OS_MAC)
  if(USE_SANDBOX)
    # Logical target used to link the cef_sandbox library.
    ADD_LOGICAL_TARGET("cef_sandbox_lib" "${CEF_SANDBOX_LIB_DEBUG}" "${CEF_SANDBOX_LIB_RELEASE}")
  endif()
else()
  # Logical target used to link the libcef library on Linux and Windows.
  # On macOS the CEF framework is loaded dynamically at startup.
  ADD_LOGICAL_TARGET("libcef_lib" "${CEF_LIB_DEBUG}" "${CEF_LIB_RELEASE}")
endif()



if(OS_MAC)
  # Main app bundle.
  set(APP_BUNDLE_DIR "${TARGET_OUT_DIR}/${TARGET}.app")

  # Variables used to configure the Info.plist file.
  set(EXECUTABLE_NAME "${TARGET}")
  set(PRODUCT_NAME "${TARGET}")

  add_custom_target(
    "${TARGET}-bundle"
    COMMAND ${CMAKE_COMMAND} -E copy_directory
      "${CEF_BINARY_DIR}/Chromium Embedded Framework.framework"
      "${APP_BUNDLE_DIR}/Contents/Frameworks/Chromium Embedded Framework.framework"
    COMMAND ${CMAKE_COMMAND} -E copy
      "${PROJECT_BINARY_DIR}/rust-cef-demo/rust-cef-demo"
      "${APP_BUNDLE_DIR}/Contents/MacOS/rust-cef-demo"
    COMMAND ${CMAKE_COMMAND} -E copy
      "${PROJECT_SOURCE_DIR}/rust-cef-demo/resources/mac/Info.plist"
      "${APP_BUNDLE_DIR}/Contents/Info.plist"
    VERBATIM
  )

  add_dependencies("${TARGET}-bundle" ${TARGET})

  foreach(_suffix_list ${CEF_HELPER_APP_SUFFIXES})
    # Convert to a list and extract the suffix values.
    string(REPLACE ":" ";" _suffix_list ${_suffix_list})
    list(GET _suffix_list 0 _name_suffix)
    list(GET _suffix_list 1 _target_suffix)
    list(GET _suffix_list 2 _plist_suffix)

    set(_helper_target "${HELPER_TARGET}${_target_suffix}")
    set(_helper_target_bundle "${_helper_target}-bundle")
    set(_helper_output_name "${HELPER_OUTPUT_NAME}${_name_suffix}")

    # Create Helper-specific variants of the helper-Info.plist file. Do this
    # manually because the configure_file command (which is executed as part of
    # MACOSX_BUNDLE_INFO_PLIST) uses global env variables and would insert the
    # wrong values with multiple targets.
    set(_helper_info_plist "${CMAKE_CURRENT_BINARY_DIR}/helper-Info${_target_suffix}.plist")
    file(READ "${PROJECT_SOURCE_DIR}/rust-cef-demo/resources/mac/helper-Info.plist" _plist_contents)
    string(REPLACE "\${EXECUTABLE_NAME}" "${_helper_output_name}" _plist_contents ${_plist_contents})
    string(REPLACE "\${PRODUCT_NAME}" "${_helper_output_name}" _plist_contents ${_plist_contents})
    string(REPLACE "\${BUNDLE_ID_SUFFIX}" "${_plist_suffix}" _plist_contents ${_plist_contents})
    file(WRITE ${_helper_info_plist} ${_plist_contents})

    add_custom_target(
      ${_helper_target_bundle}
      COMMAND ${CMAKE_COMMAND} -E copy
        "${PROJECT_BINARY_DIR}/helper_process"
        "${TARGET_OUT_DIR}/${_helper_output_name}.app/Contents/${_helper_output_name}"
      COMMAND 
      COMMAND ${CMAKE_COMMAND} -E copy
        ${_helper_info_plist}
        "${TARGET_OUT_DIR}/${_helper_output_name}.app/Contents/Info.plist"
      VERBATIM
    )

    add_dependencies("${_helper_target}-bundle" ${HELPER_TARGET})
    add_dependencies("${TARGET}-bundle" ${_helper_target_bundle})

    add_custom_command(
      TARGET "${TARGET}-bundle"
      POST_BUILD
      COMMAND ${CMAKE_COMMAND} -E copy_directory
        "${PROJECT_BINARY_DIR}/${CMAKE_BUILD_TYPE}/${_helper_output_name}.app"
        "${APP_BUNDLE_DIR}/Contents/Frameworks/${_helper_output_name}.app"
      VERBATIM
      )
  endforeach()
endif()