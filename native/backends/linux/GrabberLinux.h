#pragma once

#ifndef GRABBER_LINUX_H
#define GRABBER_LINUX_H

#include "Grabber.h"

class GrabberLinux : public Grabber {
public:
  explicit GrabberLinux(QObject *parent = nullptr);
  ~GrabberLinux() override = default;

  std::vector<CapturedFrame> captureAll() override;
  bool authorize() override;

private:
  std::vector<CapturedFrame> captureStandard();
#if defined(Q_OS_LINUX)
  std::vector<CapturedFrame> captureWayland();
  bool resetWaylandScreenshotPermission();
  QString requestWaylandScreenshot();
#endif
};

using GrabberUnix = GrabberLinux;

extern "C" Grabber *createUnixEngine(QObject *parent);

#endif // GRABBER_LINUX_H
