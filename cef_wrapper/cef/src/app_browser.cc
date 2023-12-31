// Copyright (c) 2017 The Chromium Embedded Framework Authors. All rights
// reserved. Use of this source code is governed by a BSD-style license that
// can be found in the LICENSE file.

#include "app_factory.h"

#include "include/cef_app.h"

// Minimal implementation of CefApp for the browser process.
class BrowserApp : public CefApp, public CefBrowserProcessHandler {
  public: BrowserApp() {}

  BrowserApp(const BrowserApp&) = delete;
  BrowserApp& operator=(const BrowserApp&) = delete;
  ~BrowserApp() = default;

  // CefApp methods:
  CefRefPtr<CefBrowserProcessHandler> GetBrowserProcessHandler() override {
    return this;
  }

  void OnBeforeCommandLineProcessing(
      const CefString& process_type,
      CefRefPtr<CefCommandLine> command_line) override {
    // Command-line flags can be modified in this callback.
    // |process_type| is empty for the browser process.
    if (process_type.empty()) {
#if defined(OS_MACOSX)
      // Disable the macOS keychain prompt. Cookies will not be encrypted.
      command_line->AppendSwitch("use-mock-keychain");
#endif
    }
  }

  // CefBrowserProcessHandler methods:
  void OnContextInitialized() override {
    // Create the browser window.
  }

 private:
  IMPLEMENT_REFCOUNTING(BrowserApp);
};

CefRefPtr<CefApp> CreateBrowserProcessApp() {
  return new BrowserApp();
}
