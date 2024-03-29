// Copyright (c) 2017 The Chromium Embedded Framework Authors. All rights
// reserved. Use of this source code is governed by a BSD-style license that
// can be found in the LICENSE file.

#include "app_factory.h"

// No CefApp for other subprocesses.
CefRefPtr<CefApp> CreateOtherProcessApp() {
  return nullptr;
}

// No CefApp for the renderer subprocess.
CefRefPtr<CefApp> CreateRendererProcessApp() {
  return nullptr;
}

