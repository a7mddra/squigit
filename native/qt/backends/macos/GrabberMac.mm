#include "GrabberMac.h"
#include <QGuiApplication>
#include <QPixmap>
#include <QScreen>

#import <CoreGraphics/CoreGraphics.h>

GrabberMac::GrabberMac(QObject *parent) : Grabber(parent) {}

bool hasMacScreenCaptureAccess() { return CGPreflightScreenCaptureAccess(); }

bool GrabberMac::authorize() {
  if (hasMacScreenCaptureAccess()) {
    return true;
  }
  CGRequestScreenCaptureAccess();
  return false;
}

std::vector<CapturedFrame> GrabberMac::captureAll() {
  std::vector<CapturedFrame> frames;

  if (!hasMacScreenCaptureAccess()) {
    CGRequestScreenCaptureAccess();
    return frames;
  }

  const auto screens = QGuiApplication::screens();
  int index = 0;

  for (QScreen *screen : screens) {
    if (!screen)
      continue;

    QPixmap pixmap = screen->grabWindow(0);

    if (pixmap.isNull()) {
      continue;
    }

    CapturedFrame frame;
    frame.image = pixmap.toImage();
    frame.geometry = screen->geometry();
    frame.devicePixelRatio = screen->devicePixelRatio();
    frame.image.setDevicePixelRatio(frame.devicePixelRatio);
    frame.name = screen->name();
    frame.index = index++;

    frames.push_back(frame);
  }

  Grabber::sortLeftToRight(frames);
  return frames;
}

extern "C" Grabber *createUnixEngine(QObject *parent) {
  return new GrabberMac(parent);
}
