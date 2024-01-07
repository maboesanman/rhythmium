
#include "client.h"

// destructor
Client::~Client() {
    _client_settings.on_paint_destroy(_client_settings.on_paint_arg);
    _client_settings.get_view_rect_destroy(_client_settings.get_view_rect_arg);
    _client_settings.on_browser_created_destroy(_client_settings.on_browser_created_arg);
    _client_settings.get_scale_factor_destroy(_client_settings.get_scale_factor_arg);
    _client_settings.get_screen_point_destroy(_client_settings.get_screen_point_arg);
}
