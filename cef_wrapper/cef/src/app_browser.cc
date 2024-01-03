// Copyright (c) 2017 The Chromium Embedded Framework Authors. All rights
// reserved. Use of this source code is governed by a BSD-style license that
// can be found in the LICENSE file.

#include "app_factory.h"

#include "include/cef_app.h"

// Minimal implementation of CefApp for the browser process.
class BrowserApp : public CefApp, public CefBrowserProcessHandler {
  public: BrowserApp(void (*app_ready)(void* callback_arg), void* app_ready_arg) {
    _app_ready = app_ready;
    _app_ready_arg = app_ready_arg;
  }

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
    _app_ready(this->_app_ready_arg);
  }

 private:
  IMPLEMENT_REFCOUNTING(BrowserApp);
  void (*_app_ready)(void* callback_arg);
  void* _app_ready_arg;
};

CefRefPtr<CefApp> CreateBrowserProcessApp(void (*app_ready)(void* app_ready_arg), void* app_ready_arg) {
  return new BrowserApp(app_ready, app_ready_arg);
}
