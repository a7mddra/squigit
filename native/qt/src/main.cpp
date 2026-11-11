#include "CaptureRuntime.h"
#include "Protocol.h"
#include "config.h"
#include <QGuiApplication>
#include <QEvent>
#include <QQuickWindow>
#include <QPointer>
#include <iostream>
#include <string>
#include <thread>
#ifdef Q_OS_WIN
#include <fcntl.h>
#include <io.h>
#include <windows.h>
#endif

class ResidentApplication : public QGuiApplication {
public:
  using QGuiApplication::QGuiApplication;

protected:
  bool event(QEvent *event) override {
    if (event->type() == QEvent::Quit)
      return true;
    return QGuiApplication::event(event);
  }
};

int main(int argc, char *argv[]) {
#ifdef Q_OS_WIN
  _setmode(_fileno(stdout), _O_BINARY);
#endif

  for (int index = 1; index < argc; ++index) {
    if (std::string(argv[index]) == "--version") {
      std::cout << "squigit-qt " << APP_VERSION << std::endl;
      return 0;
    }
    std::cerr << "CAPTURE_NATIVE_ERROR: unknown argument: " << argv[index] << std::endl;
    return 1;
  }

#if QT_VERSION >= QT_VERSION_CHECK(5, 14, 0)
  QGuiApplication::setHighDpiScaleFactorRoundingPolicy(
      Qt::HighDpiScaleFactorRoundingPolicy::PassThrough);
#endif

#ifdef Q_OS_WIN
  HMODULE user32 = LoadLibraryW(L"user32.dll");
  if (user32) {
    using SetProcessDpiAwarenessContextFn = BOOL(WINAPI *)(HANDLE);
    auto fn = reinterpret_cast<SetProcessDpiAwarenessContextFn>(
        GetProcAddress(user32, "SetProcessDpiAwarenessContext"));
    if (fn) {
      fn(reinterpret_cast<HANDLE>(-4));
    } else {
      HMODULE shcore = LoadLibraryW(L"Shcore.dll");
      if (shcore) {
        using SetProcessDpiAwarenessFn = HRESULT(WINAPI *)(int);
        auto fn2 = reinterpret_cast<SetProcessDpiAwarenessFn>(
            GetProcAddress(shcore, "SetProcessDpiAwareness"));
        if (fn2) {
          constexpr int PROCESS_PER_MONITOR_DPI_AWARE = 2;
          fn2(PROCESS_PER_MONITOR_DPI_AWARE);
        }
        FreeLibrary(shcore);
      } else {
        using SetProcessDPIAwareFn = BOOL(WINAPI *)();
        auto fn3 = reinterpret_cast<SetProcessDPIAwareFn>(
            GetProcAddress(user32, "SetProcessDPIAware"));
        if (fn3)
          fn3();
      }
    }
    FreeLibrary(user32);
  }
#endif

#ifdef Q_OS_LINUX
  qputenv("QT_QPA_PLATFORM", "xcb");
#endif

  QQuickWindow::setDefaultAlphaBuffer(true);
  ResidentApplication app(argc, argv);
  app.setApplicationName("Squigit");
  app.setDesktopFileName("squigit");
  app.setOrganizationName(ORG_NAME);
  app.setApplicationVersion(APP_VERSION);
  app.setQuitOnLastWindowClosed(false);
  CaptureRuntime runtime;
  if (!runtime.ready()) {
    std::cerr << "CAPTURE_NATIVE_ERROR: could not initialize the Qt runtime" << std::endl;
    return 1;
  }
  auto input = std::thread([&app, target = QPointer<CaptureRuntime>(&runtime)]() {
    std::string line;
    while (std::getline(std::cin, line)) {
      const auto message = QByteArray::fromStdString(line);
      QMetaObject::invokeMethod(&app, [target, message]() {
        if (target)
          target->acceptMessage(message);
      }, Qt::QueuedConnection);
    }
    QMetaObject::invokeMethod(&app, [target]() {
      if (target)
        target->shutdown();
    }, Qt::QueuedConnection);
  });
  sendEvent({{"type", "ready"}});
  const int result = app.exec();
  input.join();
  return result;
}
