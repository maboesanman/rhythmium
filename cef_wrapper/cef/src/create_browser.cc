
#include "include/cef_browser.h"

#include "create_browser.h"
#include "client.h"


int create_browser(
  void (*on_paint) ( void* arg, const void* buffer, int width, int height ),
  void* on_paint_arg
) {
  CefRefPtr<CefBrowser> browser;
  {
    CefWindowInfo window_info;
    CefBrowserSettings browser_settings;

    browser_settings.windowless_frame_rate = 60;
    window_info.SetAsWindowless(nullptr);

    ClientSettings client_settings;
    client_settings.on_paint = on_paint;
    client_settings.on_paint_arg = on_paint_arg;

    browser = CefBrowserHost::CreateBrowserSync(
      window_info,
      new Client(client_settings),
      "https://media1.giphy.com/media/v1.Y2lkPTc5MGI3NjExMWJtd2gwNDh2ZHMzb294N2pwdjI3MXkyZGZyZDBjbWxtMTFxOWowMCZlcD12MV9pbnRlcm5hbF9naWZfYnlfaWQmY3Q9Zw/oEXpKfMSx5WXC/giphy.mp4",
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
