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

bool Client::GetScreenInfo(CefRefPtr<CefBrowser> browser, CefScreenInfo& screen_info) {

  auto old_scale_factor = screen_info.device_scale_factor;
  auto new_scale_factor = old_scale_factor;
  _client_settings.get_scale_factor(_client_settings.get_scale_factor_arg, &new_scale_factor);

  if (new_scale_factor == old_scale_factor) {
    return false;
  }

  int w;
  int h;
  _client_settings.get_view_rect(_client_settings.get_view_rect_arg, &w, &h);

  screen_info.Set(
    new_scale_factor,
    32,
    0,
    false,
    CefRect(0, 0, w, h),
    CefRect(0, 0, w, h)
  );

  return true;
}

bool Client::GetScreenPoint(CefRefPtr<CefBrowser> browser, int viewX, int viewY, int& screenX, int& screenY) {
  _client_settings.get_screen_point(_client_settings.get_screen_point_arg, viewX, viewY, &screenX, &screenY);
  return true;
}
