#include "client.h"
#include "include/capi/cef_browser_capi.h"
#include "libcef_dll/ctocpp/browser_ctocpp.h"


void Client::OnAfterCreated(CefRefPtr<CefBrowser> browser) {
  CefBrowser* browser_ptr = browser.get();
  cef_browser_t* c_browser = CefBrowserCToCpp::Unwrap(browser_ptr);
  _client_settings.on_browser_created(_client_settings.on_browser_created_arg, c_browser);
}

bool Client::DoClose(CefRefPtr<CefBrowser> browser) {
  return false;
}

void Client::OnBeforeClose(CefRefPtr<CefBrowser> browser) {
}
