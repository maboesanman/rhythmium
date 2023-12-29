// Copyright (c) 2017 The Chromium Embedded Framework Authors. All rights
// reserved. Use of this source code is governed by a BSD-style license that
// can be found in the LICENSE file.

#include "shared/main.h"

// Main program entry point function.
#if defined(OS_WIN)
int APIENTRY wWinMain(HINSTANCE hInstance,
                      HINSTANCE hPrevInstance,
                      LPTSTR lpCmdLine,
                      int nCmdShow) {
  UNREFERENCED_PARAMETER(hPrevInstance);
  UNREFERENCED_PARAMETER(lpCmdLine);
  UNREFERENCED_PARAMETER(nCmdShow);
  return shared::wWinMain(hInstance);
}
#else

// mark as extern "C" to be usable from rust
// int main(int argc, char* argv[]) {
//   // return shared::main(argc, argv);
//   return 0;
// }

extern "C" int main_ffi(int argc, char* argv[]) {
  return shared::main(argc, argv);
}
#endif
