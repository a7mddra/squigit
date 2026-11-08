#pragma once

#ifndef GRABBER_WIN_H
#define GRABBER_WIN_H

#include "Grabber.h"

#ifdef Q_OS_WIN
#include <windows.h>
#endif

class GrabberWin : public Grabber {
public:
  explicit GrabberWin(QObject *parent = nullptr);
  ~GrabberWin() override;

  std::vector<CapturedFrame> captureAll() override;

#ifdef Q_OS_WIN
private:
  ULONG_PTR m_gdiplusToken;
  bool m_gdiplusReady;
#endif
};

extern "C" Grabber *createWindowsEngine(QObject *parent);

#endif // GRABBER_WIN_H
