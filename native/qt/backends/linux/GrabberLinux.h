#pragma once

#ifndef GRABBER_LINUX_H
#define GRABBER_LINUX_H

#include "Grabber.h"
#include <QEventLoop>
#include <QPointer>

class GrabberLinux : public Grabber {
public:
  explicit GrabberLinux(QObject *parent = nullptr);
  ~GrabberLinux() override = default;

  std::vector<CapturedFrame> captureAll() override;
  bool authorize() override;
  void cancel() override;

private:
  std::vector<CapturedFrame> captureStandard();
#if defined(Q_OS_LINUX)
  std::vector<CapturedFrame> captureWayland();
  bool resetWaylandScreenshotPermission();
  QString requestWaylandScreenshot();
  QPointer<QEventLoop> m_requestLoop;
  QString m_requestPath;
  bool m_requestCancelled = false;
#endif
};

using GrabberUnix = GrabberLinux;

extern "C" Grabber *createUnixEngine(QObject *parent);

#endif // GRABBER_LINUX_H
