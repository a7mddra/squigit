#pragma once

#include <QImage>
#include <QObject>
#include <QPolygonF>
#include <QRectF>
#include <QTemporaryFile>
#include <QUrl>
#include <QVariantList>
#include <QtQml/qqml.h>
#include <memory>

class CaptureEngine : public QObject {
  Q_OBJECT
  QML_ELEMENT
  QML_SINGLETON
  Q_PROPERTY(QUrl backgroundSource READ backgroundSource NOTIFY backgroundSourceChanged)
  Q_PROPERTY(QString captureMode READ captureMode WRITE setCaptureMode NOTIFY captureModeChanged)
  Q_PROPERTY(int displayIndex READ displayIndex WRITE setDisplayIndex NOTIFY displayIndexChanged)
  Q_PROPERTY(QRectF selection READ selection NOTIFY selectionChanged)
  Q_PROPERTY(bool hasSelection READ hasSelection NOTIFY selectionChanged)
  Q_PROPERTY(bool ocrPending READ ocrPending NOTIFY ocrChanged)
  Q_PROPERTY(bool ocrVisible READ ocrVisible NOTIFY ocrChanged)
  Q_PROPERTY(QVariantList textBoxes READ textBoxes NOTIFY ocrChanged)
  Q_PROPERTY(QString status READ status NOTIFY ocrChanged)
  Q_PROPERTY(qreal pixelRatio READ pixelRatio CONSTANT)

public:
  explicit CaptureEngine(QObject *parent = nullptr);
  void setBackgroundImage(const QImage &image, qreal devicePixelRatio);
  QUrl backgroundSource() const { return m_backgroundSource; }
  QString captureMode() const { return m_captureMode; }
  void setCaptureMode(const QString &mode);
  void setAutoOcr(bool enabled) { m_autoOcr = enabled; }
  int displayIndex() const { return m_displayIndex; }
  void setDisplayIndex(int index);
  QRectF selection() const { return m_selection; }
  bool hasSelection() const { return !m_selection.isEmpty(); }
  bool ocrPending() const { return m_ocrPending; }
  bool ocrVisible() const { return m_ocrVisible; }
  QVariantList textBoxes() const { return m_textBoxes; }
  QString status() const { return m_ocrReady && !m_ocrVisible ? QString() : m_status; }
  qreal pixelRatio() const { return m_devicePixelRatio; }
  void acceptOcrResponse(const QByteArray &line);

  Q_INVOKABLE QString ensureBackgroundFileFallback();
  Q_INVOKABLE void cancel();
  Q_INVOKABLE void finishSquiggleCapture(const QVariantList &points);
  Q_INVOKABLE void finishTraditionalCapture(QPointF start, QPointF end);
  Q_INVOKABLE void save();
  Q_INVOKABLE void toggleOcr();
  Q_INVOKABLE void setSelection(const QRectF &logicalRect);
  Q_INVOKABLE void copyText();

signals:
  void backgroundSourceChanged();
  void captureModeChanged();
  void displayIndexChanged();
  void selectionChanged();
  void ocrChanged();

private:
  struct Character {
    int begin;
    int end;
    qreal left;
    qreal right;
    QPointF center;
  };
  struct TextRegion {
    QString text;
    QPolygonF box;
    QList<Character> characters;
  };
  void select(const QRectF &logicalRect);
  void recognize();
  void rebuildTextBoxes();
  QString saveImage(const QImage &rgb) const;
  void fail();
  QImage m_backgroundImage;
  std::unique_ptr<QTemporaryFile> m_backgroundFile;
  std::unique_ptr<QTemporaryFile> m_ocrFile;
  QUrl m_backgroundSource;
  QRectF m_selection;
  qreal m_devicePixelRatio = 1.0;
  QString m_captureMode = "squiggle";
  int m_displayIndex = 0;
  bool m_ocrPending = false;
  bool m_autoOcr = false;
  bool m_ocrReady = false;
  bool m_ocrVisible = false;
  QList<TextRegion> m_regions;
  QVariantList m_textBoxes;
  QString m_status;
  QString m_requestId;
  int m_requestSequence = 0;
};
