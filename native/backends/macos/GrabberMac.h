#pragma once

#ifndef GRABBER_MAC_H
#define GRABBER_MAC_H

#include "Grabber.h"

class GrabberMac : public Grabber {
public:
  explicit GrabberMac(QObject *parent = nullptr);
  ~GrabberMac() override = default;

  std::vector<CapturedFrame> captureAll() override;
  bool authorize() override;
};

bool hasMacScreenCaptureAccess();
extern "C" Grabber *createUnixEngine(QObject *parent);

#endif // GRABBER_MAC_H
