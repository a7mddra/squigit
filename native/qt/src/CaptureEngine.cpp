#include "CaptureEngine.h"
#include <QDateTime>
#include <QClipboard>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <limits>
#include <QtMath>
#include <QDir>
#include <QFileInfo>
#include <QGuiApplication>
#include <QStandardPaths>
#include <cmath>
#include <iostream>

namespace {

QString outputDirectory() {
  const QString configured =
      qEnvironmentVariable("SQUIGIT_OUTPUT_DIR").trimmed();
  if (!configured.isEmpty()) {
    return configured;
  }
  const QStringList pictures =
      QStandardPaths::standardLocations(QStandardPaths::PicturesLocation);
  if (pictures.isEmpty()) {
    return QDir::temp().filePath("squigit");
  }
  return pictures.first() + "/squigit";
}

} // namespace

CaptureEngine::CaptureEngine(QObject *parent) : QObject(parent) {}

void CaptureEngine::setBackgroundImage(const QImage &image,
                                       qreal devicePixelRatio) {
  m_backgroundImage = image;
  m_devicePixelRatio = devicePixelRatio > 0 ? devicePixelRatio : 1.0;
  m_backgroundSource =
      QUrl(QString("image://backgrounds/%1").arg(m_displayIndex));
  emit backgroundSourceChanged();
}

void CaptureEngine::setCaptureMode(const QString &mode) {
  if (m_captureMode != mode) {
    m_captureMode = mode;
    emit captureModeChanged();
  }
}

void CaptureEngine::setDisplayIndex(int index) {
  if (m_displayIndex != index) {
    m_displayIndex = index;
    emit displayIndexChanged();
  }
}

QString CaptureEngine::ensureBackgroundFileFallback() {
  if (m_backgroundImage.isNull()) {
    return m_backgroundSource.toString();
  }

  if (!m_backgroundFile) {
    auto file = std::make_unique<QTemporaryFile>(
        QDir::temp().filePath("squigit-capture-background-XXXXXX.bmp"));
    if (!file->open() || !m_backgroundImage.save(file.get(), "BMP")) {
      return QString();
    }
    file->close();
    m_backgroundFile = std::move(file);
  }

  m_backgroundSource = QUrl::fromLocalFile(m_backgroundFile->fileName());
  emit backgroundSourceChanged();
  return m_backgroundSource.toString();
}

void CaptureEngine::cancel() {
  std::cout << "CAPTURE_CANCELLED" << std::endl;
  QGuiApplication::exit(2);
}

void CaptureEngine::finishSquiggleCapture(const QVariantList &points) {
  if (points.isEmpty()) {
    fail();
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

  select(boundingRect);
}

void CaptureEngine::finishTraditionalCapture(QPointF start, QPointF end) {
  QRectF selectionRect = QRectF(start, end).normalized();

  if (selectionRect.width() < 1 || selectionRect.height() < 1) {
    fail();
    return;
  }

  select(selectionRect);
}

void CaptureEngine::select(const QRectF &logicalRect) {
  const QRect pixels(qFloor(logicalRect.x() * m_devicePixelRatio),
                     qFloor(logicalRect.y() * m_devicePixelRatio),
                     qCeil(logicalRect.width() * m_devicePixelRatio),
                     qCeil(logicalRect.height() * m_devicePixelRatio));
  const QRect clipped = pixels.intersected(m_backgroundImage.rect());
  if (clipped.isEmpty()) return;
  m_selectedImage = m_backgroundImage.copy(clipped).convertToFormat(QImage::Format_RGB888);
  m_selectedImage.setDevicePixelRatio(1.0);
  m_selection = QRectF(clipped.x() / m_devicePixelRatio,
                       clipped.y() / m_devicePixelRatio,
                       clipped.width() / m_devicePixelRatio,
                       clipped.height() / m_devicePixelRatio);
  emit selectionChanged();
}

void CaptureEngine::recognize() {
  if (!hasSelection() || m_ocrPending) return;
  m_ocrFile = std::make_unique<QTemporaryFile>(QDir::temp().filePath("squigit-ocr-XXXXXX.png"));
  if (!m_ocrFile->open() || !m_selectedImage.save(m_ocrFile.get(), "PNG")) {
    m_status = "Could not prepare the selected image";
    emit ocrChanged();
    return;
  }
  m_ocrFile->close();
  m_requestId = QString("%1-%2").arg(m_displayIndex).arg(++m_requestSequence);
  m_ocrPending = true;
  m_status = "Recognizing text...";
  emit ocrChanged();
  const QJsonObject request{{"id", m_requestId}, {"image", m_ocrFile->fileName()}};
  std::cout << "OCR_REQUEST:" << QJsonDocument(request).toJson(QJsonDocument::Compact).toStdString() << std::endl;
}

void CaptureEngine::acceptOcrResponse(const QByteArray &line) {
  const QJsonObject response = QJsonDocument::fromJson(line).object();
  if (!m_ocrPending || response["id"].toString() != m_requestId) return;
  m_ocrPending = false;
  m_ocrFile.reset();
  const QString error = response["error"].toString();
  if (!error.isEmpty()) {
    m_status = error;
  } else {
    m_textBoxes = response["boxes"].toArray().toVariantList();
    m_status = m_textBoxes.isEmpty() ? "No text found" : "Drag to select text. Ctrl+C copies the selection; Ctrl+Shift+C copies all text.";
  }
  emit ocrChanged();
}

void CaptureEngine::copyText() {
  QStringList lines;
  for (const auto &box : m_textBoxes) lines.append(box.toMap()["text"].toString());
  QGuiApplication::clipboard()->setText(lines.join("\n"));
}

QString CaptureEngine::saveImage(const QImage &rgb) const {
  const QDir base(outputDirectory());
  if (!base.exists() && !base.mkpath(".")) {
    return QString();
  }

  const QString stamp =
      QDateTime::currentDateTime().toString("yyyyMMdd-HHmmss-zzz");
  QString path = base.filePath(QString("squigit-%1.png").arg(stamp));
  int suffix = 1;
  while (QFileInfo::exists(path)) {
    path = base.filePath(QString("squigit-%1-%2.png").arg(stamp).arg(suffix));
    suffix += 1;
  }

  if (!rgb.save(path, "PNG")) {
    return QString();
  }
  return path;
}

void CaptureEngine::save() {
  if (!hasSelection()) return;
  const QString savedPath = saveImage(m_selectedImage);
  if (savedPath.isEmpty()) {
    std::cerr << "CAPTURE_NATIVE_ERROR: could not write the captured image"
              << std::endl;
    fail();
    return;
  }

  std::cout << "SQUIGIT_SAVED:" << savedPath.toStdString() << std::endl;
  std::cout.flush();

  QGuiApplication::exit(0);
}

void CaptureEngine::fail() {
  std::cout << "CAPTURE_FAIL" << std::endl;
  std::cout.flush();

  QGuiApplication::exit(1);
}
