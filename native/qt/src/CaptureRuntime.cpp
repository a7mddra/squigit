#include "CaptureRuntime.h"
#include "CaptureEngine.h"
#include "annotation/AnnotationCanvas.h"
#include "DisplayGuard.h"
#include "Grabber.h"
#include "Protocol.h"
#include "WindowEffects.h"
#include "menu/RadialMenu.h"

#include <QEvent>
#include <QGuiApplication>
#include <QJsonDocument>
#include <QKeyEvent>
#include <QQmlContext>
#include <QQuickImageProvider>
#include <QScreen>
#include <QTimer>
#include <QUuid>
#include <iostream>

#ifdef Q_OS_MAC
#include <objc/message.h>
#include <objc/runtime.h>
#endif

extern "C" Grabber *createWindowsEngine(QObject *parent);
extern "C" Grabber *createUnixEngine(QObject *parent);

class BackgroundImageProvider : public QQuickImageProvider {
public:
  BackgroundImageProvider() : QQuickImageProvider(QQuickImageProvider::Image) {}
  void setImage(int index, const QImage &image) {
    m_images.insert(QString::number(index), image);
  }
  void clear() { m_images.clear(); }
  QImage requestImage(const QString &id, QSize *size, const QSize &) override {
    const auto image = m_images.value(id);
    if (size)
      *size = image.size();
    return image;
  }
private:
  QHash<QString, QImage> m_images;
};

class CaptureWindowFocus : public QObject {
public:
  CaptureWindowFocus(QQuickWindow *window, CaptureEngine *controller)
      : QObject(window), m_window(window), m_controller(controller) {
    window->installEventFilter(this);
    m_escapeTimer.setSingleShot(true);
    m_escapeTimer.setInterval(650);
    connect(&m_escapeTimer, &QTimer::timeout, this, [this]() {
      m_escapeHeld = true;
      m_controller->cancel();
    });
  }

protected:
  bool eventFilter(QObject *object, QEvent *event) override {
    const auto type = event->type();
#ifdef Q_OS_LINUX
    if (type == QEvent::CursorChange && m_window->isVisible())
      m_window->update();
#endif
    if (type == QEvent::Hide || type == QEvent::WindowDeactivate) {
      m_escapeTimer.stop();
      m_escapePressed = false;
      m_escapeHeld = false;
    }
    if (m_window->isVisible() && m_window->property("ownsInput").toBool() &&
        (type == QEvent::ShortcutOverride || type == QEvent::KeyPress ||
         type == QEvent::KeyRelease)) {
      auto *key = static_cast<QKeyEvent *>(event);
      if (key->key() == Qt::Key_Escape && key->modifiers() == Qt::NoModifier) {
        if (type == QEvent::ShortcutOverride) {
          key->accept();
          return true;
        }
        if (key->isAutoRepeat())
          return true;
        if (type == QEvent::KeyPress && !m_escapePressed) {
          m_escapePressed = true;
          m_escapeHeld = false;
          m_escapeTimer.start();
          QMetaObject::invokeMethod(m_window, "escapePressed", Qt::DirectConnection);
        } else if (type == QEvent::KeyRelease && m_escapePressed) {
          m_escapeTimer.stop();
          m_escapePressed = false;
          if (!m_escapeHeld)
            QMetaObject::invokeMethod(m_window, "escapeTapped",
                                      Qt::DirectConnection);
          m_escapeHeld = false;
        }
        return true;
      }
    }
    if (m_controller->hasSelection() &&
        (type == QEvent::MouseButtonPress ||
         type == QEvent::MouseButtonDblClick || type == QEvent::TouchBegin)) {
      m_window->requestActivate();
      if (!m_window->setKeyboardGrabEnabled(true)) {
        std::cerr << "CAPTURE_NATIVE_ERROR: could not restore keyboard input after clicking the capture overlay"
                  << std::endl;
      }
    }
    return QObject::eventFilter(object, event);
  }

private:
  QQuickWindow *m_window;
  CaptureEngine *m_controller;
  QTimer m_escapeTimer;
  bool m_escapePressed = false;
  bool m_escapeHeld = false;
};

static void applyPlatformWindowHacks(QQuickWindow *window) {
  disableWindowAnimations(window);

#ifdef Q_OS_MAC
  WId nativeId = window->winId();
  id nsView = reinterpret_cast<id>(nativeId);
  if (nsView) {
    id nsWindow =
        ((id (*)(id, SEL))objc_msgSend)(nsView, sel_registerName("window"));
    if (nsWindow) {
      ((void (*)(id, SEL, BOOL))objc_msgSend)(
          nsWindow, sel_registerName("setHasShadow:"), NO);
      ((void (*)(id, SEL, long))objc_msgSend)(nsWindow,
                                              sel_registerName("setLevel:"), 5);
    }
  }
#endif
}

CaptureRuntime::CaptureRuntime(QObject *parent)
    : QObject(parent), m_captureComponent(&m_qml) {
#ifdef Q_OS_WIN
  m_grabber = createWindowsEngine(this);
#else
  m_grabber = createUnixEngine(this);
#endif
  qmlRegisterType<AnnotationCanvas>("SquigitAnnotation", 1, 0, "AnnotationCanvas");
  qmlRegisterUncreatableType<AnnotationDocument>("SquigitAnnotation", 1, 0, "AnnotationDocument", "Owned by the capture");
  m_backgrounds = new BackgroundImageProvider();
  m_qml.addImageProvider("backgrounds", m_backgrounds);
  m_captureComponent.loadUrl(QUrl("qrc:/views/CaptureWindow.qml"));
  if (m_captureComponent.isError())
    std::cerr << m_captureComponent.errorString().toStdString() << std::endl;
  m_menu = std::make_unique<RadialMenu>(&m_qml, this);
  connect(m_menu.get(), &RadialMenu::actionActivated, this, &CaptureRuntime::startAction);
  connect(m_menu.get(), &RadialMenu::tickRequested, this,
          []() { sendEvent({{"type", "menu_tick"}}); });
  auto *displayGuard = new DisplayGuard(this);
  connect(displayGuard, &DisplayGuard::changed, this, [this]() {
    ++m_generation;
    finishCapture("cancelled");
  });
}

CaptureRuntime::~CaptureRuntime() {
  qDeleteAll(m_windows);
  qDeleteAll(m_controllers);
  for (const auto &session : m_retired.keys())
    releaseCapture(session);
}

bool CaptureRuntime::ready() const {
  return m_grabber && m_menu && m_menu->ready() && m_captureComponent.isReady();
}

void CaptureRuntime::acceptMessage(const QByteArray &line) {
  QJsonParseError error;
  const auto document = QJsonDocument::fromJson(line, &error);
  if (error.error != QJsonParseError::NoError || !document.isObject()) {
    sendEvent({{"type", "error"}, {"message", "Invalid Qt command"}});
    return;
  }
  const auto message = document.object();
  const auto type = message["type"].toString();
  if (type == "show_menu") {
    showMenu();
  } else if (type == "hide_menu") {
    m_menu->hide();
  } else if (type == "shutdown") {
    shutdown();
  } else if (type == "capture_released") {
    releaseCapture(message["session"].toString());
  } else if (type == "ocr_result" || type == "qr_result") {
    const auto controllers = m_controllers;
    for (auto *controller : controllers)
      controller->acceptResponse(line);
  } else {
    sendEvent({{"type", "error"}, {"message", "Unknown Qt command"}});
  }
}

void CaptureRuntime::shutdown() {
  ++m_generation;
  finishCapture("cancelled");
  m_grabber->cancel();
  QCoreApplication::exit(0);
}

void CaptureRuntime::showMenu() {
  ++m_generation;
  finishCapture("cancelled");
  m_menu->show();
}

void CaptureRuntime::startAction(const QString &name) {
  const auto action = captureAction(name);
  if (action.id.isEmpty())
    return;
  finishCapture("cancelled");
  const auto generation = ++m_generation;
  m_menu->hide();
  QTimer::singleShot(350, this, [this, action, generation]() {
    if (generation == m_generation)
      beginCapture(action, generation);
  });
}

void CaptureRuntime::beginCapture(const CaptureAction &action, quint64 generation) {
  m_action = action;
  m_session = QUuid::createUuid().toString(QUuid::WithoutBraces);
  m_selectionFinished = false;
  const bool savesCapture = action.id == "snapshot" || action.id == "ask-google" ||
                            action.id == "circle-to-search";
  if (savesCapture && action.outputDirectory.isEmpty()) {
    finishCapture("failed", "The capture destination is missing");
    return;
  }
  const auto frames = m_grabber->captureAll();
  if (generation != m_generation)
    return;
  if (frames.empty()) {
    finishCapture("failed", "No display frames were captured");
    return;
  }
  const auto screens = qGuiApp->screens();
  for (const auto &frame : frames) {
    if (generation != m_generation)
      return;
    QScreen *target = nullptr;
    for (auto *screen : screens) {
      if (screen->name() == frame.name) {
        target = screen;
        break;
      }
    }
    if (!target) {
      for (auto *screen : screens) {
        if (screen->geometry() == frame.geometry) {
          target = screen;
          break;
        }
      }
    }
    m_backgrounds->setImage(frame.index, frame.image);
    auto *controller = new CaptureEngine(this);
    controller->setDisplayIndex(frame.index);
    controller->setAction(action);
    controller->setBackgroundImage(frame.image, frame.devicePixelRatio);
    m_controllers.append(controller);
    connect(controller, &CaptureEngine::completed, this,
            [this](const QString &status, const QString &detail) { finishCapture(status, detail); });
    connect(controller, &CaptureEngine::askGoogleSubmitted, this,
            [this](const QString &prompt, const QString &path) { finishCapture("ask_google", path, prompt); });
    connect(controller, &CaptureEngine::imageExported, this,
            [this, generation](const QString &path) {
              if (generation == m_generation && !m_session.isEmpty())
                sendEvent({{"type", "image_exported"}, {"session", m_session}, {"path", path}});
            });
    connect(controller, &CaptureEngine::selectionStarted, this, [this, controller]() {
      for (int index = 0; index < m_controllers.size(); ++index) {
        const bool chosen = m_controllers[index] == controller;
        m_windows[index]->setProperty("ownsInput", chosen);
        if (chosen)
          m_chosenWindow = m_windows[index];
      }
    });
    connect(controller, &CaptureEngine::selectionChanged, this, [this, controller, generation]() {
      if (!controller->hasSelection() || m_selectionFinished)
        return;
      m_selectionFinished = true;
      QTimer::singleShot(0, this, [this, generation]() {
        if (generation != m_generation || !m_chosenWindow)
          return;
        m_chosenWindow->raise();
        m_chosenWindow->requestActivate();
        if (!m_chosenWindow->setKeyboardGrabEnabled(true)) {
          std::cerr << "CAPTURE_NATIVE_ERROR: could not transfer the keyboard grab to the selected display; keeping the other overlays visible" << std::endl;
          return;
        }
        for (auto *window : m_windows) {
          if (window != m_chosenWindow)
            window->hide();
        }
      });
    });
    const QVariantMap properties{{"controller", QVariant::fromValue(controller)},
                                 {"ownsInput", m_windows.isEmpty()}};
    auto *object = m_captureComponent.createWithInitialProperties(properties);
    auto *window = qobject_cast<QQuickWindow *>(object);
    if (!window) {
      delete object;
      finishCapture("failed", m_captureComponent.errorString());
      return;
    }
    window->QObject::setParent(this);
    m_windows.append(window);
    if (target)
      window->setScreen(target);
    window->setGeometry(target ? target->geometry() : frame.geometry);
    applyPlatformWindowHacks(window);
    new CaptureWindowFocus(window, controller);
    window->showFullScreen();
  }
  QTimer::singleShot(0, this, [this, generation]() {
    if (generation != m_generation || m_chosenWindow || m_windows.isEmpty())
      return;
    auto *window = m_windows.front();
    window->raise();
    window->requestActivate();
    if (!window->setKeyboardGrabEnabled(true))
      std::cerr << "CAPTURE_NATIVE_ERROR: could not grab the keyboard for the capture overlay" << std::endl;
  });
}

void CaptureRuntime::finishCapture(const QString &status, const QString &detail,
                                   const QString &prompt) {
  if (m_action.id.isEmpty())
    return;
  m_grabber->cancel();
  ++m_generation;
  for (auto *controller : m_controllers)
    controller->retire();
  for (auto *window : m_windows) {
    window->setKeyboardGrabEnabled(false);
    window->hide();
  }
  m_retired.insert(m_session, {m_windows, m_controllers});
  m_windows.clear();
  m_controllers.clear();
  m_chosenWindow.clear();
  m_backgrounds->clear();
  QJsonObject event{{"type", "capture_finished"}, {"session", m_session},
                    {"action", m_action.id}, {"status", status}};
  if (status == "saved")
    event.insert("path", detail);
  else if (status == "qr_open" || status == "ocr_open")
    event.insert("url", detail);
  else if (status == "ask_google")
    event.insert("prompt", prompt);
  else if (!detail.isEmpty())
    event.insert("error", detail);
  m_action = {};
  m_session.clear();
  sendEvent(event);
}

void CaptureRuntime::releaseCapture(const QString &session) {
  const auto retired = m_retired.take(session);
  qDeleteAll(retired.windows);
  qDeleteAll(retired.controllers);
}
