/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

#include <QCommandLineParser>
#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QQmlComponent>
#include <QQmlContext>
#include <QQuickWindow>
#include <QScreen>
#include <vector>

#include "config.h"
#include "controller/CaptureController.h"
#include "core/CaptureMode.h"
#include "core/ScreenGrabber.h"

#ifdef Q_OS_WIN
#include <dwmapi.h>
#include <windows.h>
#endif

#ifdef Q_OS_MAC
#include <objc/message.h>
#include <objc/runtime.h>
#endif

extern "C" ScreenGrabber *createWindowsEngine(QObject *parent);
extern "C" ScreenGrabber *createUnixEngine(QObject *parent);

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
        ((id(*)(id, SEL))objc_msgSend)(nsView, sel_registerName("window"));
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

#if QT_VERSION < QT_VERSION_CHECK(6, 0, 0)
  QCoreApplication::setAttribute(Qt::AA_EnableHighDpiScaling);
  QCoreApplication::setAttribute(Qt::AA_UseHighDpiPixmaps);
#endif
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

  app.setApplicationName(APP_NAME);
  app.setDesktopFileName("squigit");
  app.setOrganizationName(ORG_NAME);
  app.setApplicationVersion(APP_VERSION);
  app.setQuitOnLastWindowClosed(true);

  QCommandLineParser parser;
  parser.setApplicationDescription("Screen capture tool with selection modes");
  parser.addHelpOption();
  parser.addVersionOption();

  QCommandLineOption freeshapeOption(
      QStringList() << "f"
                    << "freeshape",
      "Use freeshape (squiggle) selection mode (default)");
  parser.addOption(freeshapeOption);

  QCommandLineOption rectangleOption(QStringList() << "r"
                                                   << "rectangle",
                                     "Use rectangle selection mode");
  parser.addOption(rectangleOption);

  parser.process(app);

  QString captureMode = "freeshape";
  if (parser.isSet(rectangleOption)) {
    captureMode = "rectangle";
  }

  ScreenGrabber *engine = nullptr;
#ifdef Q_OS_WIN
  engine = createWindowsEngine(&app);
#else
  engine = createUnixEngine(&app);
#endif

  if (!engine) {
    return 1;
  }

  std::vector<CapturedFrame> frames = engine->captureAll();

  if (frames.empty()) {
    return 1;
  }

  QList<QScreen *> qtScreens = app.screens();
  QQmlApplicationEngine qmlEngine;
  std::vector<CaptureController *> controllers;
  std::vector<QQuickWindow *> windows;

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

    auto *controller = new CaptureController(&app);
    controller->setDisplayIndex(frame.index);
    controller->setCaptureMode(captureMode);
    controller->setBackgroundImage(frame.image, frame.devicePixelRatio);
    controller->setDisplayGeometry(frame.geometry);
    controllers.push_back(controller);

    QQmlComponent component(&qmlEngine,
                            QUrl("qrc:/CaptureQml/qml/CaptureWindow.qml"));

    if (component.isError()) {
      return 1;
    }

    QVariantMap properties;
    properties["controller"] = QVariant::fromValue(controller);

    QObject *obj = component.createWithInitialProperties(properties);
    QQuickWindow *window = qobject_cast<QQuickWindow *>(obj);

    if (!window) {
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
    window->showFullScreen();
  }

  return app.exec();
}
