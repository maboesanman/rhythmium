#include "client.h"
#include "include/cef_browser.h"
#include "include/cef_render_handler.h"

void Client::GetViewRect(CefRefPtr<CefBrowser> browser, CefRect& rect) {
  _client_settings.get_view_rect(_client_settings.get_view_rect_arg, &rect.width, &rect.height);
}

void Client::OnPaint(
  CefRefPtr<CefBrowser> browser,
  CefRenderHandler::PaintElementType type,
  const CefRenderHandler::RectList& dirtyRects,
  const void* buffer,
  int width,
  int height
) {
  auto arg = _client_settings.on_paint_arg;
  _client_settings.on_paint(arg, buffer, width, height);
}
