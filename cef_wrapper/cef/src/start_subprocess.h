// Copyright (c) 2017 The Chromium Embedded Framework Authors. All rights
// reserved. Use of this source code is governed by a BSD-style license that
// can be found in the LICENSE file.

#pragma once

// Entry point function shared by executable targets.
// returns 0 if execution should continue or non-zero to terminate early.
// this is intended to be called at the beginning of main functions, exiting early if it returns non-zero.
// if it returns 0, then it will also eventually call app_ready() when the app is established and the create_browser() function can be called.
extern "C" int try_start_subprocess(int argc, char **argv, void (*app_ready)(void* callback_arg), void* callback_arg);
