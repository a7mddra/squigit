/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

#include "ScreenGrabber.h"
#include <QGuiApplication>
#include <QPixmap>
#include <QScreen>
#include <iostream>
#if defined(Q_OS_LINUX)
#include <QDBusConnection>
#include <QDBusInterface>
#include <QDBusMessage>
#include <QDBusObjectPath>
#include <QDBusReply>
#include <QEventLoop>
#include <QFile>
#include <QTimer>
#include <QUrl>
#include <QUuid>
#include <QWindow>
#endif
#include <cmath>

#if defined(Q_OS_LINUX)
class PortalHelper : public QObject {
  Q_OBJECT
public:
  QString savedUri;
  bool success = false;

public slots:
  void handleResponse(uint response, const QVariantMap &results) {
    if (response == 0) {
      savedUri = results.value("uri").toString();
      success = !savedUri.isEmpty();
    } else if (response == 2) {
      QDBusMessage dropPerm = QDBusMessage::createMethodCall(
          "org.freedesktop.impl.portal.PermissionStore",
          "/org/freedesktop/impl/portal/PermissionStore",
          "org.freedesktop.impl.portal.PermissionStore", "DeletePermission");
      dropPerm << "screenshot"
               << "screenshot"
               << "squigit";
      QDBusConnection::sessionBus().call(dropPerm);

      std::cout << "CAPTURE_DENIED" << std::endl;
      std::cout.flush();
      success = false;
    } else {
      success = false;
    }
    emit finished();
  }

signals:
  void finished();
};
#endif

class ScreenGrabberUnix : public ScreenGrabber {
public:
  ScreenGrabberUnix(QObject *parent = nullptr) : ScreenGrabber(parent) {}

  std::vector<CapturedFrame> captureAll() override {
#if defined(Q_OS_LINUX)
    QString sessionType = qgetenv("XDG_SESSION_TYPE").toLower();
    if (sessionType == "wayland") {
      return captureWayland();
    } else {
      return captureStandard();
    }
#else
    return captureStandard();
#endif
  }

private:
  std::vector<CapturedFrame> captureStandard() {
    std::vector<CapturedFrame> frames;
    const auto screens = QGuiApplication::screens();
    int index = 0;

    for (QScreen *screen : screens) {
      if (!screen)
        continue;
      QPixmap pixmap = screen->grabWindow(0);
      if (pixmap.isNull())
        continue;

      CapturedFrame frame;
      frame.image = pixmap.toImage();
      frame.geometry = screen->geometry();
      frame.devicePixelRatio = screen->devicePixelRatio();
      frame.image.setDevicePixelRatio(frame.devicePixelRatio);
      frame.name = screen->name();
      frame.index = index++;
      frames.push_back(frame);
    }
    ScreenGrabber::sortLeftToRight(frames);
    return frames;
  }

#if defined(Q_OS_LINUX)
  std::vector<CapturedFrame> captureWayland() {
    std::vector<CapturedFrame> frames;

    QWindow dummyWindow;
    dummyWindow.setFlags(Qt::FramelessWindowHint |
                         Qt::WindowTransparentForInput);
    dummyWindow.resize(1, 1);
    dummyWindow.create();
    QString parentWindow =
        QString("x11:%1").arg((quintptr)dummyWindow.winId(), 0, 16);

    QDBusInterface portal("org.freedesktop.portal.Desktop",
                          "/org/freedesktop/portal/desktop",
                          "org.freedesktop.portal.Screenshot");

    if (!portal.isValid()) {
      return frames;
    }

    QString token =
        QUuid::createUuid().toString().remove('{').remove('}').remove('-');
    QVariantMap options;
    options["handle_token"] = token;
    options["interactive"] = false;

    PortalHelper helper;
    QEventLoop loop;

    QString sender = QDBusConnection::sessionBus().baseService();
    sender = sender.mid(1).replace('.', '_');
    QString expectedPath =
        QString("/org/freedesktop/portal/desktop/request/%1/%2")
            .arg(sender)
            .arg(token);

    QDBusConnection::sessionBus().connect(
        "org.freedesktop.portal.Desktop", expectedPath,
        "org.freedesktop.portal.Request", "Response", &helper,
        SLOT(handleResponse(uint, QVariantMap)));
    QObject::connect(&helper, &PortalHelper::finished, &loop,
                     &QEventLoop::quit);

    QDBusReply<QDBusObjectPath> reply =
        portal.call("Screenshot", parentWindow, options);
    if (!reply.isValid()) {
      return frames;
    }

    QTimer::singleShot(60000000, &loop, &QEventLoop::quit);

    loop.exec();

    if (!helper.success) {
      return frames;
    }

    QString localPath = QUrl(helper.savedUri).toLocalFile();
    QImage fullDesktop(localPath);

    QFile::remove(localPath);

    if (fullDesktop.isNull()) {
      return frames;
    }

    QRect logicalBounds;
    for (QScreen *screen : QGuiApplication::screens()) {
      logicalBounds = logicalBounds.united(screen->geometry());
    }

    double scaleFactor = 1.0;
    if (logicalBounds.width() > 0) {
      scaleFactor = (double)fullDesktop.width() / (double)logicalBounds.width();
    }

    int index = 0;
    for (QScreen *screen : QGuiApplication::screens()) {
      QRect geo = screen->geometry();

      int cropX = std::round((geo.x() - logicalBounds.x()) * scaleFactor);
      int cropY = std::round((geo.y() - logicalBounds.y()) * scaleFactor);
      int cropW = std::round(geo.width() * scaleFactor);
      int cropH = std::round(geo.height() * scaleFactor);

      if (cropX < 0)
        cropX = 0;
      if (cropY < 0)
        cropY = 0;
      if (cropX + cropW > fullDesktop.width())
        cropW = fullDesktop.width() - cropX;
      if (cropY + cropH > fullDesktop.height())
        cropH = fullDesktop.height() - cropY;

      QImage screenImg = fullDesktop.copy(cropX, cropY, cropW, cropH);
      screenImg.setDevicePixelRatio(scaleFactor);

      CapturedFrame frame;
      frame.image = screenImg;
      frame.geometry = geo;
      frame.devicePixelRatio = scaleFactor;
      frame.name = screen->name();
      frame.index = index++;

      frames.push_back(frame);
    }

    ScreenGrabber::sortLeftToRight(frames);
    return frames;
  }
#endif
};

#if defined(Q_OS_LINUX)
#include "GrabberLinux.moc"

extern "C" ScreenGrabber *createUnixEngine(QObject *parent) {
  return new ScreenGrabberUnix(parent);
}
#endif
