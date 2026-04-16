/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

#include "CaptureController.h"
#include <QDateTime>
#include <QDir>
#include <QGuiApplication>
#include <QTemporaryFile>
#include <cmath>
#include <iostream>

CaptureController::CaptureController(QObject *parent) : QObject(parent) {}

void CaptureController::setBackgroundImage(const QImage &image,
                                           qreal devicePixelRatio) {
  m_backgroundImage = image;
  m_devicePixelRatio = devicePixelRatio > 0 ? devicePixelRatio : 1.0;

  QString tempPath =
      QDir::temp().filePath(QString("capture_bg_%1.png").arg(m_displayIndex));

  if (m_backgroundImage.save(tempPath, "PNG")) {
    m_backgroundSource = QUrl::fromLocalFile(tempPath);
    emit backgroundSourceChanged();
  }
}

void CaptureController::setCaptureMode(const QString &mode) {
  if (m_captureMode != mode) {
    m_captureMode = mode;
    emit captureModeChanged();
  }
}

void CaptureController::setDisplayIndex(int index) {
  if (m_displayIndex != index) {
    m_displayIndex = index;
    emit displayIndexChanged();
  }
}

void CaptureController::cancel() { emitFailure(); }

void CaptureController::finishSquiggleCapture(const QVariantList &points) {
  if (points.isEmpty()) {
    emitFailure();
    return;
  }

  qreal minX = std::numeric_limits<qreal>::max();
  qreal maxX = std::numeric_limits<qreal>::lowest();
  qreal minY = std::numeric_limits<qreal>::max();
  qreal maxY = std::numeric_limits<qreal>::lowest();

  const qreal margin = 10.0;

  for (const QVariant &v : points) {
    QPointF pt = v.toPointF();
    minX = qMin(minX, pt.x());
    maxX = qMax(maxX, pt.x());
    minY = qMin(minY, pt.y());
    maxY = qMax(maxY, pt.y());
  }

  minX -= margin;
  maxX += margin;
  minY -= margin;
  maxY += margin;

  QRectF boundingRect(minX, minY, maxX - minX, maxY - minY);

  cropAndSave(boundingRect);
}

void CaptureController::finishRectCapture(QPointF start, QPointF end) {
  QRectF selectionRect = QRectF(start, end).normalized();

  if (selectionRect.width() < 1 || selectionRect.height() < 1) {
    emitFailure();
    return;
  }

  cropAndSave(selectionRect);
}

void CaptureController::cropAndSave(const QRectF &logicalRect) {
  int physX = qRound(logicalRect.x() * m_devicePixelRatio);
  int physY = qRound(logicalRect.y() * m_devicePixelRatio);
  int physW = qRound(logicalRect.width() * m_devicePixelRatio);
  int physH = qRound(logicalRect.height() * m_devicePixelRatio);

  physX = qMax(0, physX);
  physY = qMax(0, physY);

  if (physX + physW > m_backgroundImage.width())
    physW = m_backgroundImage.width() - physX;
  if (physY + physH > m_backgroundImage.height())
    physH = m_backgroundImage.height() - physY;

  if (physW <= 0 || physH <= 0) {
    emitFailure();
    return;
  }

  QImage cropped = m_backgroundImage.copy(physX, physY, physW, physH);
  cropped.setDevicePixelRatio(1.0);

  QString timestamp =
      QDateTime::currentDateTime().toString("yyyyMMdd_hhmmss_zzz");
  QString finalPath =
      QDir::temp().filePath(QString("squigit_capture_%1.png").arg(timestamp));

  if (cropped.save(finalPath, "PNG", -1)) {
    emitSuccess(finalPath);
  } else {
    emitFailure();
  }
}

void CaptureController::emitSuccess(const QString &path) {
  std::cout << "CAPTURE_SUCCESS" << std::endl;
  std::cout << "DISPLAY_GEO:" << m_displayGeometry.x() << ","
            << m_displayGeometry.y() << "," << m_displayGeometry.width() << ","
            << m_displayGeometry.height() << std::endl;
  std::cout << path.toStdString() << std::endl;
  std::cout.flush();

  emit captureCompleted(path);
  QGuiApplication::exit(0);
}

void CaptureController::emitFailure() {
  std::cout << "CAPTURE_FAIL" << std::endl;
  std::cout.flush();

  emit captureFailed();
  QGuiApplication::exit(1);
}
