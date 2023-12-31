// Copyright (c) 2017 The Chromium Embedded Framework Authors. All rights
// reserved. Use of this source code is governed by a BSD-style license that
// can be found in the LICENSE file.

#pragma once

#include "include/base/cef_build.h"

// Entry point function shared by executable targets.
// returns 0 if execution should continue or non-zero to terminate early.
// this is intended to be called at the beginning of main functions, exiting early if it returns non-zero.
extern "C" int try_start_subprocess(int argc, char* argv[]);
