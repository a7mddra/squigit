#include <QByteArray>
#include <QGuiApplication>
#include <QHash>
#include <QQmlApplicationEngine>
#include <QQmlComponent>
#include <QQmlContext>
#include <QQuickImageProvider>
#include <QQuickWindow>
#include <QScreen>
#include <QString>
#include <QTimer>
#include <atomic>
#include <chrono>
#include <iostream>
#include <string>
#include <thread>
#include <QJsonDocument>
#include <vector>

#include "CaptureEngine.h"
#include "Grabber.h"
#include "config.h"

#ifdef Q_OS_MAC
#include "GrabberMac.h"
#endif

#ifdef Q_OS_WIN
#include <dwmapi.h>
#include <fcntl.h>
#include <io.h>
#include <windows.h>
#endif

#ifdef Q_OS_MAC
#include <objc/message.h>
#include <objc/runtime.h>
#endif

extern "C" Grabber *createWindowsEngine(QObject *parent);
extern "C" Grabber *createUnixEngine(QObject *parent);

class BackgroundImageProvider : public QQuickImageProvider {
public:
  BackgroundImageProvider() : QQuickImageProvider(QQuickImageProvider::Image) {}

  void setImage(int displayIndex, const QImage &image) {
    m_images.insert(QString::number(displayIndex), image);
  }

  QImage requestImage(const QString &id, QSize *size,
                      const QSize &requestedSize) override {
    Q_UNUSED(requestedSize);
    const QImage image = m_images.value(id);
    if (size) {
      *size = image.size();
    }
    return image;
  }

private:
  QHash<QString, QImage> m_images;
};

static bool isTimingEnabled() {
  static const bool enabled = []() {
    const QByteArray raw = qgetenv("SQUIGIT_CAPTURE_TIMING");
    if (raw.isEmpty()) {
      return false;
    }
    const QString normalized = QString::fromUtf8(raw).trimmed().toLower();
    return normalized == "1" || normalized == "true" || normalized == "yes" ||
           normalized == "on";
  }();
  return enabled;
}

static void logTimingStage(const char *stage,
                           const std::chrono::steady_clock::time_point &start) {
  if (!isTimingEnabled()) {
    return;
  }

  const auto elapsedMs = std::chrono::duration_cast<std::chrono::milliseconds>(
                             std::chrono::steady_clock::now() - start)
                             .count();
  std::cerr << "TIMING " << stage << " " << elapsedMs << "ms" << std::endl;
}

static void applyPlatformWindowHacks(QQuickWindow *window) {
#ifdef Q_OS_WIN
  HWND hwnd = reinterpret_cast<HWND>(window->winId());
  BOOL attrib = TRUE;
  DwmSetWindowAttribute(hwnd, DWMWA_TRANSITIONS_FORCEDISABLED, &attrib,
                        sizeof(attrib));
#endif

#ifdef Q_OS_MAC
  WId nativeId = window->winId();
  id nsView = reinterpret_cast<id>(nativeId);
  if (nsView) {
    id nsWindow =
        ((id (*)(id, SEL))objc_msgSend)(nsView, sel_registerName("window"));
    if (nsWindow) {
      ((void (*)(id, SEL, long))objc_msgSend)(
          nsWindow, sel_registerName("setAnimationBehavior:"), 2);
      ((void (*)(id, SEL, BOOL))objc_msgSend)(
          nsWindow, sel_registerName("setHasShadow:"), NO);
      ((void (*)(id, SEL, long))objc_msgSend)(nsWindow,
                                              sel_registerName("setLevel:"), 5);
    }
  }
#endif
}

int main(int argc, char *argv[]) {
#ifdef Q_OS_WIN
  _setmode(_fileno(stdout), _O_BINARY);
#endif

  bool squiggleRequested = false;
  bool traditionalRequested = false;
  bool autoOcrRequested = false;
  bool authorizeRequested = false;
  bool preflightRequested = false;

  for (int index = 1; index < argc; ++index) {
    const std::string argument = argv[index];
    if (argument == "--version") {
      std::cout << "squigit-qt " << APP_VERSION << std::endl;
      return 0;
    }
    if (argument == "--squiggle") {
      squiggleRequested = true;
    } else if (argument == "--auto-ocr") {
      autoOcrRequested = true;
    } else if (argument == "--traditional") {
      traditionalRequested = true;
    } else if (argument == "--authorize") {
      authorizeRequested = true;
    } else if (argument == "--preflight") {
      preflightRequested = true;
    } else {
      std::cerr << "CAPTURE_NATIVE_ERROR: unknown argument: " << argument
                << std::endl;
      return 1;
    }
  }

  if (squiggleRequested && traditionalRequested) {
    std::cerr << "CAPTURE_NATIVE_ERROR: --traditional and --squiggle are "
                 "mutually exclusive"
              << std::endl;
    return 1;
  }
  if (authorizeRequested && (squiggleRequested || traditionalRequested)) {
    std::cerr << "CAPTURE_NATIVE_ERROR: --authorize cannot be combined with "
                 "a capture mode"
              << std::endl;
    return 1;
  }

  if (preflightRequested) {
#ifdef Q_OS_MAC
    const bool granted = hasMacScreenCaptureAccess();
#else
    const bool granted = true;
#endif
    std::cout << (granted ? "CAPTURE_PERMISSION_GRANTED"
                          : "CAPTURE_PERMISSION_DENIED")
              << std::endl;
    return 0;
  }

  const auto timingStart = std::chrono::steady_clock::now();

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

  QGuiApplication app(argc, argv);

  app.setApplicationName("Squigit");
  app.setDesktopFileName("squigit");
  app.setOrganizationName(ORG_NAME);
  app.setApplicationVersion(APP_VERSION);
  app.setQuitOnLastWindowClosed(true);

  const QString captureMode = traditionalRequested ? "traditional" : "squiggle";

  Grabber *engine = nullptr;
#ifdef Q_OS_WIN
  engine = createWindowsEngine(&app);
#else
  engine = createUnixEngine(&app);
#endif

  if (!engine) {
    std::cerr
        << "CAPTURE_NATIVE_ERROR: failed to create platform screen grabber"
        << std::endl;
    return 1;
  }

  if (authorizeRequested) {
    if (!engine->authorize()) {
      return 2;
    }
    std::cout << "CAPTURE_AUTHORIZED" << std::endl;
    return 0;
  }

  std::vector<CapturedFrame> frames = engine->captureAll();
  logTimingStage("capture_all_done", timingStart);

  if (frames.empty()) {
    std::cerr << "CAPTURE_NATIVE_ERROR: no frames captured from display backend"
              << std::endl;
    return 1;
  }

  QList<QScreen *> qtScreens = app.screens();
  QQmlApplicationEngine qmlEngine;
  auto *backgroundProvider = new BackgroundImageProvider();
  qmlEngine.addImageProvider("backgrounds", backgroundProvider);

  QQmlComponent component(&qmlEngine, QUrl("qrc:/views/CaptureWindow.qml"));
  if (component.isError()) {
    const auto errors = component.errors();
    for (const QQmlError &err : errors) {
      std::cerr << "CAPTURE_NATIVE_ERROR: QML component error: "
                << err.toString().toStdString() << std::endl;
    }
    return 1;
  }
  logTimingStage("qml_component_ready", timingStart);

  std::vector<CaptureEngine *> controllers;
  std::vector<QQuickWindow *> windows;
  QQuickWindow *chosenWindow = nullptr;
  QQuickWindow *keyboardWindow = nullptr;
  bool selectionFinished = false;
  std::atomic<bool> firstFrameLogged{false};

  for (const auto &frame : frames) {
    QScreen *targetScreen = nullptr;
    for (QScreen *s : qtScreens) {
      if (s->name() == frame.name) {
        targetScreen = s;
        break;
      }
    }
    if (!targetScreen) {
      for (QScreen *s : qtScreens) {
        if (s->geometry() == frame.geometry) {
          targetScreen = s;
          break;
        }
      }
    }

    backgroundProvider->setImage(frame.index, frame.image);

    auto *controller = new CaptureEngine(&app);
    controller->setDisplayIndex(frame.index);
    controller->setCaptureMode(captureMode);
    controller->setAutoOcr(autoOcrRequested);
    controller->setBackgroundImage(frame.image, frame.devicePixelRatio);
    controllers.push_back(controller);
    QObject::connect(controller, &CaptureEngine::selectionStarted, &app, [&, controller]() {
      for (size_t i = 0; i < controllers.size(); ++i) {
        const bool selected = controllers[i] == controller;
        windows[i]->setProperty("ownsInput", selected);
        if (selected)
          chosenWindow = windows[i];
      }
    });
    QObject::connect(controller, &CaptureEngine::selectionChanged, &app, [&, controller]() {
      if (!controller->hasSelection() || selectionFinished) return;
      selectionFinished = true;
      QTimer::singleShot(0, &app, [&]() {
        if (keyboardWindow)
          keyboardWindow->setKeyboardGrabEnabled(false);
        for (auto *window : windows) {
          if (window != chosenWindow)
            window->hide();
        }
        keyboardWindow = chosenWindow;
        keyboardWindow->raise();
        keyboardWindow->requestActivate();
        keyboardWindow->setKeyboardGrabEnabled(true);
      });
    });

    QVariantMap properties;
    properties["controller"] = QVariant::fromValue(controller);
    properties["ownsInput"] = controllers.size() == 1;

    QObject *obj = component.createWithInitialProperties(properties);
    QQuickWindow *window = qobject_cast<QQuickWindow *>(obj);

    if (!window) {
      std::cerr << "CAPTURE_NATIVE_ERROR: QML root is not a QQuickWindow"
                << std::endl;
      return 1;
    }

    windows.push_back(window);

    if (targetScreen) {
      window->setScreen(targetScreen);
      window->setGeometry(targetScreen->geometry());
    } else {
      window->setGeometry(frame.geometry);
    }

    applyPlatformWindowHacks(window);
    QObject::connect(
        window, &QQuickWindow::frameSwapped, window,
        [timingStart, &firstFrameLogged]() {
          bool expected = false;
          if (firstFrameLogged.compare_exchange_strong(expected, true)) {
            logTimingStage("first_frame_swapped", timingStart);
          }
        });
    window->showFullScreen();
  }

  logTimingStage("windows_shown", timingStart);

  QTimer::singleShot(0, &app, [&]() {
    if (chosenWindow)
      return;
    keyboardWindow = windows.front();
    keyboardWindow->raise();
    keyboardWindow->requestActivate();
    keyboardWindow->setKeyboardGrabEnabled(true);
  });

  std::thread([&app, controllers]() {
    std::string line;
    while (std::getline(std::cin, line)) {
      const QByteArray response = QByteArray::fromStdString(line);
      QMetaObject::invokeMethod(&app, [controllers, response]() {
        for (auto *controller : controllers) controller->acceptOcrResponse(response);
      }, Qt::QueuedConnection);
    }
    QMetaObject::invokeMethod(&app, &QCoreApplication::quit, Qt::QueuedConnection);
  }).detach();
  return app.exec();
}
