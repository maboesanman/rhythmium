
#pragma once

// #include "include/capi/cef_browser_capi.h"

extern "C" struct ClientSettings {
  void (*on_paint) ( void* arg, int dirty_count, const void* dirty_start, const void* buffer, int width, int height );
  void* on_paint_arg;

  void (*get_view_rect) ( void* arg, int* width, int* height );
  void* get_view_rect_arg;

  void (*on_browser_created) ( void* arg, void* browser );
  void* on_browser_created_arg;

  void (*get_scale_factor) ( void* arg, float* scale_factor);
  void* get_scale_factor_arg;

  void (*get_screen_point) ( void* arg, int view_x, int view_y, int* screen_x, int* screen_y );
  void* get_screen_point_arg;
};

extern "C" int create_browser(ClientSettings client_settings);

extern "C" void do_message_loop_work();
