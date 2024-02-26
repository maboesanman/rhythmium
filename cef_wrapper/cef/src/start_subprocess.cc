// Copyright (c) 2017 The Chromium Embedded Framework Authors. All rights
// reserved. Use of this source code is governed by a BSD-style license that
// can be found in the LICENSE file.

#include "start_subprocess.h"

#include "include/base/cef_logging.h"
#include "include/wrapper/cef_library_loader.h"

#include "app_factory.h"
#include "subprocess_util.h"


// Entry point function for all processes.
int try_start_subprocess(int argc, char* argv[]) {
  #if defined(OS_MACOSX)
    if (!InitMacMainProcess(argc, argv, false))
      return 1;

    return 0;
  #endif
  
  void* sandbox_info = nullptr;

  #if defined(OS_WIN) && defined(CEF_USE_SANDBOX)
    // Manage the life span of the sandbox information object. This is necessary
    // for sandbox support on Windows. See cef_sandbox_win.h for complete details.
    CefScopedSandboxInfo scoped_sandbox;
    sandbox_info = scoped_sandbox.sandbox_info();
  #endif

  return 0;

  // Provide CEF with command-line arguments.
  CefMainArgs main_args(argc, argv);

  CefRefPtr<CefApp> app;
  #if defined(OS_MACOSX)
    // macos fires them from the bundle, so we skip all this.
    app = CreateBrowserProcessApp();
  #else
    // Create a temporary CommandLine object.
    CefRefPtr<CefCommandLine> command_line = CreateCommandLine(main_args);

    // Create a CefApp of the correct process type.
    switch (GetProcessType(command_line)) {
      case PROCESS_TYPE_BROWSER:
        app = CreateBrowserProcessApp();
        break;
      case PROCESS_TYPE_RENDERER:
        app = CreateRendererProcessApp();
        break;
      case PROCESS_TYPE_OTHER:
        app = CreateOtherProcessApp();
        break;
    }

    // CEF applications have multiple sub-processes (render, plugin, GPU, etc)
    // that share the same executable. This function checks the command-line and,
    // if this is a sub-process, executes the appropriate logic.
    int exit_code = CefExecuteProcess(main_args, app, nullptr);
    if (exit_code >= 0) {
      // The sub-process has completed so return here.
      return exit_code;
    }
  #endif

  // Specify CEF global settings here.
  CefSettings settings;
  settings.windowless_rendering_enabled = true;

  #if !defined(CEF_USE_SANDBOX)
    settings.no_sandbox = true;
  #endif

  // Initialize CEF.
  CefInitialize(main_args, settings, app, sandbox_info);

  return 0;
}
