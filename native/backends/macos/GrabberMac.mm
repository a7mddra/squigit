#include "GrabberMac.h"
#include <QGuiApplication>
#include <QPixmap>
#include <QScreen>
#include <iostream>

#import <CoreGraphics/CoreGraphics.h>

class ScopedAudioMuteIpc {
public:
  explicit ScopedAudioMuteIpc(bool enabled = true) : m_enabled(enabled) {
    if (m_enabled) {
      std::cout << "AUDIO_MUTE" << std::endl;
      std::cout.flush();
    }
  }

  ~ScopedAudioMuteIpc() {
    if (m_enabled) {
      std::cout << "AUDIO_UNMUTE" << std::endl;
      std::cout.flush();
    }
  }

private:
  bool m_enabled;
};

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

  {
    ScopedAudioMuteIpc audioMuteScope;
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
  }

  Grabber::sortLeftToRight(frames);
  return frames;
}

extern "C" Grabber *createUnixEngine(QObject *parent) {
  return new GrabberMac(parent);
}
