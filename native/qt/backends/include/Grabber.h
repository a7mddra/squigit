#pragma once

#ifndef GRABBER_H
#define GRABBER_H

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

class Grabber : public QObject {
  Q_OBJECT

public:
  explicit Grabber(QObject *parent = nullptr) : QObject(parent) {}
  virtual ~Grabber() = default;
  virtual std::vector<CapturedFrame> captureAll() = 0;
  virtual void cancel() {}
  virtual bool authorize() { return !captureAll().empty(); }
  static void sortLeftToRight(std::vector<CapturedFrame> &frames) {
    std::sort(frames.begin(), frames.end(),
              [](const CapturedFrame &a, const CapturedFrame &b) {
                return a.geometry.x() < b.geometry.x();
              });
  }
};

#endif // GRABBER_H
