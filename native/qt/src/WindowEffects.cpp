#include "WindowEffects.h"

#include <QWindow>

#ifdef Q_OS_WIN
#include <windows.h>
#include <dwmapi.h>
#endif
#ifdef Q_OS_MAC
#include <objc/message.h>
#include <objc/runtime.h>
#endif

void disableWindowAnimations(QWindow *window) {
#ifdef Q_OS_WIN
  HWND hwnd = reinterpret_cast<HWND>(window->winId());
  BOOL disabled = TRUE;
  DwmSetWindowAttribute(hwnd, DWMWA_TRANSITIONS_FORCEDISABLED, &disabled,
                        sizeof(disabled));
#elif defined(Q_OS_MAC)
  id nsView = reinterpret_cast<id>(window->winId());
  if (nsView) {
    id nsWindow =
        ((id (*)(id, SEL))objc_msgSend)(nsView, sel_registerName("window"));
    if (nsWindow)
      ((void (*)(id, SEL, long))objc_msgSend)(
          nsWindow, sel_registerName("setAnimationBehavior:"), 2);
  }
#else
  Q_UNUSED(window);
#endif
}
