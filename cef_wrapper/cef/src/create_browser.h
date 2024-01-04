
#pragma once

extern "C" int create_browser(
  void (*on_paint) ( void* arg, const void* buffer, int width, int height ),
  void* on_paint_arg
);

extern "C" void do_message_loop_work();
