#include "client.h"
#include "include/cef_browser.h"
#include "include/cef_render_handler.h"

bool Client::GetRootScreenRect(CefRefPtr<CefBrowser> browser, CefRect& rect) {
  return false;
}

bool Client::GetScreenInfo(CefRefPtr<CefBrowser> browser, CefScreenInfo& screen_info) {
  return false;
}

bool Client::GetScreenPoint(CefRefPtr<CefBrowser> browser, int viewX, int viewY, int& screenX, int& screenY) {
  return false;
}

void Client::GetTouchHandleSize(CefRefPtr<CefBrowser> browser, cef_horizontal_alignment_t orientation, CefSize& size) {

}

void Client::GetViewRect(CefRefPtr<CefBrowser> browser, CefRect& rect) {
  
}

void OnPaint(
  CefRefPtr<CefBrowser> browser,
  CefRenderHandler::PaintElementType type,
  const CefRenderHandler::RectList& dirtyRects,
  const void* buffer,
  int width,
  int height
) {

}
