
#include "include/cef_browser.h"

#include "create_browser.h"
#include "client.h"

#include "libcef_dll/ctocpp/browser_ctocpp.h"

int create_browser(ClientSettings client_settings) {
  CefRefPtr<CefBrowser> browser;
  {
    CefWindowInfo window_info;
    CefBrowserSettings browser_settings;

    browser_settings.windowless_frame_rate = 60;
    window_info.SetAsWindowless(nullptr);

    browser = CefBrowserHost::CreateBrowserSync(
      window_info,
      new Client(client_settings),
      "https://webglsamples.org/blob/blob.html",
      // "https://www.spacejam.com/1996/jam.html",
      browser_settings,
      nullptr,
      nullptr
    );
  }
  return 0;
}

void do_message_loop_work() {
  CefDoMessageLoopWork();
}
