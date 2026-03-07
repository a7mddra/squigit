/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

#pragma once

#ifndef CAPTURECONTROLLER_H
#define CAPTURECONTROLLER_H

#include <QImage>
#include <QObject>
#include <QPointF>
#include <QRect>
#include <QRectF>
#include <QUrl>
#include <QVariantList>
#include <QtQml/qqml.h>

/**
 * @brief Bridge between QML canvas UI and C++ capture backend.
 *
 * Handles the actual image cropping and saving logic, exposing
 * methods to QML for completing or canceling captures.
 */
class CaptureController : public QObject {
  Q_OBJECT
  QML_ELEMENT
  QML_SINGLETON

  Q_PROPERTY(QUrl backgroundSource READ backgroundSource NOTIFY
                 backgroundSourceChanged)
  Q_PROPERTY(QString captureMode READ captureMode WRITE setCaptureMode NOTIFY
                 captureModeChanged)
  Q_PROPERTY(int displayIndex READ displayIndex WRITE setDisplayIndex NOTIFY
                 displayIndexChanged)

public:
  explicit CaptureController(QObject *parent = nullptr);
  ~CaptureController() override = default;

  void setBackgroundImage(const QImage &image, qreal devicePixelRatio);
  void setDisplayGeometry(const QRect &geometry) {
    m_displayGeometry = geometry;
  }

  QUrl backgroundSource() const { return m_backgroundSource; }
  QString captureMode() const { return m_captureMode; }
  void setCaptureMode(const QString &mode);
  int displayIndex() const { return m_displayIndex; }
  void setDisplayIndex(int index);

  Q_INVOKABLE void cancel();
  Q_INVOKABLE void finishSquiggleCapture(const QVariantList &points);
  Q_INVOKABLE void finishRectCapture(QPointF start, QPointF end);

signals:
  void backgroundSourceChanged();
  void captureModeChanged();
  void displayIndexChanged();
  void captureCompleted(const QString &path);
  void captureFailed();

private:
  void cropAndSave(const QRectF &logicalRect);
  void emitSuccess(const QString &path);
  void emitFailure();

  QImage m_backgroundImage;
  QUrl m_backgroundSource;
  qreal m_devicePixelRatio = 1.0;
  QString m_captureMode = "freeshape";
  int m_displayIndex = 0;
  QRect m_displayGeometry;
};

#endif // CAPTURECONTROLLER_H
