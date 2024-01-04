// Copyright (c) 2017 The Chromium Embedded Framework Authors. All rights
// reserved. Use of this source code is governed by a BSD-style license that can
// be found in the LICENSE file.

#include "process_helper_mac.h"

#include "include/cef_app.h"
#include "include/wrapper/cef_library_loader.h"

#include "app_factory.h"
#include "subprocess_util.h"

// When generating projects with CMake the CEF_USE_SANDBOX value will be defined
// automatically. Pass -DUSE_SANDBOX=OFF to the CMake command-line to disable
// use of the sandbox.
#if defined(CEF_USE_SANDBOX)
#include "include/cef_sandbox_mac.h"
#endif


// Entry point function for sub-processes.
int main(int argc, char* argv[]) {
  if(!InitMacProcess(argc, argv, true))
    return 1;

  // Provide CEF with command-line arguments.
  CefMainArgs main_args(argc, argv);

  // Create a temporary CommandLine object.
  CefRefPtr<CefCommandLine> command_line = CreateCommandLine(main_args);

  // Create a CefApp of the correct process type. The browser process is handled
  // by start_subprocess.cc.
  CefRefPtr<CefApp> app;
  switch (GetProcessType(command_line)) {
    case PROCESS_TYPE_RENDERER:
      app = CreateRendererProcessApp();
      break;
    case PROCESS_TYPE_OTHER:
      app = CreateOtherProcessApp();
      break;
    default:
      break;
  }

  // Execute the sub-process.
  return CefExecuteProcess(main_args, app, nullptr);
}
