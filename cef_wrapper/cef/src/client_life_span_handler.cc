#include "client.h"

Client::Client() {}

void Client::OnAfterCreated(CefRefPtr<CefBrowser> browser) {
}

bool Client::DoClose(CefRefPtr<CefBrowser> browser) {
  return false;
}

void Client::OnBeforeClose(CefRefPtr<CefBrowser> browser) {
}
