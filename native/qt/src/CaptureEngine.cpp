#include "CaptureEngine.h"
#include <QDateTime>
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

void CaptureEngine::cancel() { emitFailure(); }

void CaptureEngine::finishSquiggleCapture(const QVariantList &points) {
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

void CaptureEngine::finishTraditionalCapture(QPointF start, QPointF end) {
  QRectF selectionRect = QRectF(start, end).normalized();

  if (selectionRect.width() < 1 || selectionRect.height() < 1) {
    emitFailure();
    return;
  }

  cropAndSave(selectionRect);
}

void CaptureEngine::cropAndSave(const QRectF &logicalRect) {
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

  const QImage rgb = cropped.convertToFormat(QImage::Format_RGB888);
  if (rgb.isNull()) {
    emitFailure();
    return;
  }
  emitSuccess(rgb);
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

void CaptureEngine::emitSuccess(const QImage &rgb) {
  const QString savedPath = saveImage(rgb);
  if (savedPath.isEmpty()) {
    std::cerr << "CAPTURE_NATIVE_ERROR: could not write the captured image"
              << std::endl;
    emitFailure();
    return;
  }

  std::cout << "SQUIGIT_SAVED:" << savedPath.toStdString() << std::endl;
  std::cout.flush();

  emit captureCompleted();
  QGuiApplication::exit(0);
}

void CaptureEngine::emitFailure() {
  std::cout << "CAPTURE_FAIL" << std::endl;
  std::cout.flush();

  emit captureFailed();
  QGuiApplication::exit(1);
}
