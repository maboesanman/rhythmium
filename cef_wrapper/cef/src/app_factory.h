// Copyright (c) 2017 The Chromium Embedded Framework Authors. All rights
// reserved. Use of this source code is governed by a BSD-style license that
// can be found in the LICENSE file.

#pragma once

#include "include/cef_app.h"

// This file declares methods that must be implemented in each executable
// target. CefApp is a global singleton that controls process-specific
// behaviors. The returned CefApp instance will be passed to CefExecuteProcess()
// and/or CefInitialize() by the program entry point implementation. On Linux
// and Windows a single executable is used for all processes. On macOS a
// separate helper executable is used for sub-processes.

// Called in the renderer sub-process to create the CefApp for that process.
CefRefPtr<CefApp> CreateRendererProcessApp();

// Called in other sub-processes to create the CefApp for that process.
CefRefPtr<CefApp> CreateOtherProcessApp();

// Called in the main process to create the CefApp for that process.
CefRefPtr<CefApp> CreateBrowserProcessApp();
