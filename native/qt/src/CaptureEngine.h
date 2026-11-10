#pragma once

#ifndef CAPTURE_ENGINE_H
#define CAPTURE_ENGINE_H

#include <QImage>
#include <QObject>
#include <QPointF>
#include <QRect>
#include <QRectF>
#include <QTemporaryFile>
#include <QUrl>
#include <QVariantList>
#include <QtQml/qqml.h>
#include <memory>

/**
 * @brief Bridge between the QML canvas UI and the captured frames.
 *
 * Handles the actual image cropping and file persistence, exposing
 * methods to QML for completing or canceling captures. On success the
 * cutout is written into the configured output folder as a PNG and the
 * absolute file path is printed on stdout (SQUIGIT_SAVED:<path>).
 */
class CaptureEngine : public QObject {
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
  explicit CaptureEngine(QObject *parent = nullptr);
  ~CaptureEngine() override = default;

  void setBackgroundImage(const QImage &image, qreal devicePixelRatio);

  QUrl backgroundSource() const { return m_backgroundSource; }
  QString captureMode() const { return m_captureMode; }
  void setCaptureMode(const QString &mode);
  int displayIndex() const { return m_displayIndex; }
  void setDisplayIndex(int index);

  Q_INVOKABLE QString ensureBackgroundFileFallback();
  Q_INVOKABLE void cancel();
  Q_INVOKABLE void finishSquiggleCapture(const QVariantList &points);
  Q_INVOKABLE void finishTraditionalCapture(QPointF start, QPointF end);

signals:
  void backgroundSourceChanged();
  void captureModeChanged();
  void displayIndexChanged();
  void captureCompleted();
  void captureFailed();

private:
  void cropAndSave(const QRectF &logicalRect);
  QString saveImage(const QImage &rgb) const;
  void emitSuccess(const QImage &rgb);
  void emitFailure();

  QImage m_backgroundImage;
  std::unique_ptr<QTemporaryFile> m_backgroundFile;
  QUrl m_backgroundSource;
  qreal m_devicePixelRatio = 1.0;
  QString m_captureMode = "squiggle";
  int m_displayIndex = 0;
};

#endif // CAPTURE_ENGINE_H
