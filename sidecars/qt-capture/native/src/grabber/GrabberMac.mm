/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

#include "ScreenGrabber.h"
#include <QGuiApplication>
#include <QOperatingSystemVersion>
#include <QPixmap>
#include <QScreen>

#import <AppKit/AppKit.h>
#import <CoreGraphics/CoreGraphics.h>

class ScreenGrabberMac : public ScreenGrabber {
public:
  ScreenGrabberMac(QObject *parent = nullptr) : ScreenGrabber(parent) {}

  std::vector<CapturedFrame> captureAll() override {
    std::vector<CapturedFrame> frames;

    if (QOperatingSystemVersion::current() >=
        QOperatingSystemVersion::MacOSCatalina) {
      if (!CGPreflightScreenCaptureAccess()) {
        CGRequestScreenCaptureAccess();

        NSAlert *alert = [[NSAlert alloc] init];
        [alert setMessageText:@"Screen Recording Permission"];
        [alert setInformativeText:
                   @"Engine requires screen recording permission to take "
                   @"screenshots.\nPlease grant permission in System Settings. "
                   @"The application will close after."];
        [alert addButtonWithTitle:@"Open System Settings"];
        [alert addButtonWithTitle:@"Cancel"];
        [alert setAlertStyle:NSAlertStyleInformational];

        NSInteger response = [alert runModal];

        if (response == NSAlertFirstButtonReturn) {
          [[NSWorkspace sharedWorkspace]
              openURL:[NSURL
                          URLWithString:
                              @"x-apple.systempreferences:com.apple.preference."
                              @"security?Privacy_ScreenRecording"]];
        }

        return frames;
      }
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

    ScreenGrabber::sortLeftToRight(frames);
    return frames;
  }
};

extern "C" ScreenGrabber *createUnixEngine(QObject *parent) {
  return new ScreenGrabberMac(parent);
}
