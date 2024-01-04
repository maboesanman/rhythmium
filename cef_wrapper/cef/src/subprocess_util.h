// Copyright (c) 2017 The Chromium Embedded Framework Authors. All rights
// reserved. Use of this source code is governed by a BSD-style license that
// can be found in the LICENSE file.

#pragma once

#include "include/cef_command_line.h"

// This file provides functionality common to all program entry point
// implementations.

// Create a new CommandLine object for use before CEF initialization.
CefRefPtr<CefCommandLine> CreateCommandLine(const CefMainArgs& main_args);

// Process types that may have different CefApp instances.
enum ProcessType {
  PROCESS_TYPE_BROWSER,
  PROCESS_TYPE_RENDERER,
  PROCESS_TYPE_OTHER,
};

// Determine the process type based on command-line arguments.
ProcessType GetProcessType(const CefRefPtr<CefCommandLine>& command_line);

#if defined(OS_MACOSX)
bool InitMacProcess(int argc, char* argv[], bool helper);
#endif
