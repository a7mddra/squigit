/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

#pragma once

#ifndef SCREENGRABBER_H
#define SCREENGRABBER_H

#include <QImage>
#include <QObject>
#include <QRect>
#include <QString>
#include <algorithm>
#include <vector>

struct CapturedFrame {
  QImage image;
  QRect geometry;
  qreal devicePixelRatio;
  int index;
  QString name;
};

class ScreenGrabber : public QObject {
  Q_OBJECT

public:
  explicit ScreenGrabber(QObject *parent = nullptr) : QObject(parent) {}
  virtual ~ScreenGrabber() = default;
  virtual std::vector<CapturedFrame> captureAll() = 0;
  static void sortLeftToRight(std::vector<CapturedFrame> &frames) {
    std::sort(frames.begin(), frames.end(),
              [](const CapturedFrame &a, const CapturedFrame &b) {
                return a.geometry.x() < b.geometry.x();
              });
  }
};

#endif // SCREENGRABBER_H
