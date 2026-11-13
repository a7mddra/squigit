#pragma once

#include <QImage>
#include <QObject>
#include <QPolygonF>
#include <QRectF>
#include <QTemporaryFile>
#include <QTimer>
#include <QUrl>
#include <QVariantList>
#include <QtQml/qqml.h>
#include <memory>
#include "actions/Actions.h"
#include "annotation/AnnotationDocument.h"

class CaptureEngine : public QObject {
  Q_OBJECT
  QML_ELEMENT
  QML_SINGLETON
  Q_PROPERTY(AnnotationDocument *annotations READ annotations CONSTANT)
  Q_PROPERTY(QUrl backgroundSource READ backgroundSource NOTIFY backgroundSourceChanged)
  Q_PROPERTY(QString captureMode READ captureMode WRITE setCaptureMode NOTIFY captureModeChanged)
  Q_PROPERTY(QString action READ action CONSTANT)
  Q_PROPERTY(int displayIndex READ displayIndex WRITE setDisplayIndex NOTIFY displayIndexChanged)
  Q_PROPERTY(QRectF selection READ selection NOTIFY selectionChanged)
  Q_PROPERTY(bool hasSelection READ hasSelection NOTIFY selectionChanged)
  Q_PROPERTY(bool ocrPending READ ocrPending NOTIFY ocrChanged)
  Q_PROPERTY(bool ocrCancelling READ ocrCancelling NOTIFY ocrChanged)
  Q_PROPERTY(bool ocrVisible READ ocrVisible NOTIFY ocrChanged)
  Q_PROPERTY(bool ocrReady READ ocrReady NOTIFY ocrChanged)
  Q_PROPERTY(bool hasOcrText READ hasOcrText NOTIFY ocrChanged)
  Q_PROPERTY(bool ocrFailed READ ocrFailed NOTIFY ocrChanged)
  Q_PROPERTY(QVariantList textBoxes READ textBoxes NOTIFY ocrChanged)
  Q_PROPERTY(QString ocrError READ ocrError NOTIFY ocrChanged)
  Q_PROPERTY(bool qrPending READ qrPending NOTIFY qrChanged)
  Q_PROPERTY(bool qrFallbackAvailable READ qrFallbackAvailable NOTIFY qrChanged)
  Q_PROPERTY(QString qrStatus READ qrStatus NOTIFY qrChanged)
  Q_PROPERTY(qreal pixelRatio READ pixelRatio CONSTANT)

public:
  explicit CaptureEngine(QObject *parent = nullptr);
  void setAction(const CaptureAction &action);
  QString action() const { return m_action.id; }
  void retire();
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
  bool ocrCancelling() const { return m_ocrCancelling; }
  bool ocrVisible() const { return m_ocrVisible; }
  bool ocrReady() const { return m_ocrReady; }
  bool hasOcrText() const;
  bool ocrFailed() const { return m_ocrFailed; }
  QVariantList textBoxes() const { return m_textBoxes; }
  QString ocrError() const { return m_ocrError; }
  bool qrPending() const { return m_qrPending; }
  bool qrFallbackAvailable() const { return m_qrFallbackAvailable; }
  QString qrStatus() const { return m_qrStatus; }
  qreal pixelRatio() const { return m_devicePixelRatio; }
  void acceptResponse(const QByteArray &line);
  AnnotationDocument *annotations() { return &m_annotations; }

  Q_INVOKABLE QString ensureBackgroundFileFallback();
  Q_INVOKABLE void cancel();
  Q_INVOKABLE void beginSelection();
  Q_INVOKABLE void finishSquiggleCapture(const QVariantList &points);
  Q_INVOKABLE void finishTraditionalCapture(QPointF start, QPointF end);
  Q_INVOKABLE void save();
  Q_INVOKABLE bool exportImage(bool annotated = true);
  Q_INVOKABLE bool copyImage(bool annotated = true);
  Q_INVOKABLE void askGoogle(const QString &prompt);
  Q_INVOKABLE void toggleOcr();
  Q_INVOKABLE void cancelOcr();
  Q_INVOKABLE void scanQr();
  Q_INVOKABLE void setSelection(const QRectF &logicalRect);
  Q_INVOKABLE void copyText();
  Q_INVOKABLE void copyTextSelection(const QString &text);
  Q_INVOKABLE void openOcrText(const QString &text, bool translate);

signals:
  void completed(const QString &status, const QString &detail);
  void askGoogleSubmitted(const QString &prompt, const QString &path);
  void imageExported(const QString &path);
  void selectionStarted();
  void backgroundSourceChanged();
  void captureModeChanged();
  void displayIndexChanged();
  void selectionChanged();
  void ocrChanged();
  void qrChanged();

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
  void completeSelection();
  void recognize();
  bool rebuildTextBoxes();
  QImage selectedImage(bool annotated = false) const;
  QString saveImage(const QImage &rgb, const QString &directory = {}) const;
  void fail();
  AnnotationDocument m_annotations;
  QImage m_backgroundImage;
  std::unique_ptr<QTemporaryFile> m_backgroundFile;
  std::unique_ptr<QTemporaryFile> m_ocrFile;
  std::unique_ptr<QTemporaryFile> m_qrFile;
  QUrl m_backgroundSource;
  QRectF m_selection;
  qreal m_devicePixelRatio = 1.0;
  QString m_captureMode = "traditional";
  CaptureAction m_action;
  QTimer m_qrTimer;
  QString m_instanceId;
  bool m_retired = false;
  int m_displayIndex = 0;
  bool m_ocrPending = false;
  bool m_ocrCancelling = false;
  bool m_captureStarted = false;
  bool m_autoOcr = false;
  bool m_ocrReady = false;
  bool m_ocrFailed = false;
  bool m_ocrVisible = false;
  QList<TextRegion> m_regions;
  QVariantList m_textBoxes;
  QString m_ocrError;
  QString m_requestId;
  int m_requestSequence = 0;
  bool m_qrPending = false;
  bool m_qrFallbackAvailable = false;
  QString m_qrStatus;
  QString m_qrRequestId;
  int m_qrRequestSequence = 0;
};
