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
#include <QFontMetricsF>
#include <QLineF>
#include <QStandardPaths>
#include <QTextBoundaryFinder>
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
  const QRectF bounds(0, 0, m_backgroundImage.width() / m_devicePixelRatio,
                      m_backgroundImage.height() / m_devicePixelRatio);
  const QRectF clipped = logicalRect.normalized().intersected(bounds);
  if (clipped.width() * m_devicePixelRatio < 1 ||
      clipped.height() * m_devicePixelRatio < 1 || clipped == m_selection)
    return;
  const bool firstSelection = !hasSelection();
  m_selection = clipped;
  const bool textChanged = rebuildTextBoxes();
  emit selectionChanged();
  if (textChanged)
    emit ocrChanged();
  if (firstSelection && m_autoOcr)
    recognize();
}

void CaptureEngine::setSelection(const QRectF &logicalRect) {
  if (hasSelection())
    select(logicalRect);
}

void CaptureEngine::toggleOcr() {
  if (!hasSelection() || m_ocrPending)
    return;
  if (!m_ocrReady) {
    recognize();
    return;
  }
  m_ocrVisible = !m_ocrVisible;
  emit ocrChanged();
}

void CaptureEngine::recognize() {
  if (!hasSelection() || m_ocrPending || m_ocrReady) return;
  m_ocrFile = std::make_unique<QTemporaryFile>(QDir::temp().filePath("squigit-ocr-XXXXXX.png"));
  QImage image = m_backgroundImage.convertToFormat(QImage::Format_RGB888);
  image.setDevicePixelRatio(1.0);
  if (!m_ocrFile->open() || !image.save(m_ocrFile.get(), "PNG")) {
    m_ocrFile.reset();
    m_status = "Could not prepare the display image. Click OCR to retry.";
    emit ocrChanged();
    return;
  }
  m_ocrFile->close();
  m_requestId = QString("%1-%2").arg(m_displayIndex).arg(++m_requestSequence);
  m_ocrPending = true;
  m_status = "Recognizing display text...";
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
    m_status = error + ". Click OCR to retry.";
  } else {
    m_regions.clear();
    for (const auto &entry : response["boxes"].toArray()) {
      const auto object = entry.toObject();
      TextRegion region;
      region.text = object["text"].toString();
      const auto points = object["box"].toArray();
      if (region.text.isEmpty() || points.size() != 4)
        continue;
      for (const auto &point : points) {
        const auto coordinates = point.toArray();
        region.box.append(QPointF(coordinates[0].toDouble(), coordinates[1].toDouble()));
      }
      QFont font = QGuiApplication::font();
      font.setPixelSize(qMax(1, qRound(QLineF(region.box[0], region.box[3]).length() * 0.85)));
      const QFontMetricsF metrics(font);
      const qreal totalWidth = metrics.horizontalAdvance(region.text);
      QTextBoundaryFinder finder(QTextBoundaryFinder::Grapheme, region.text);
      int begin = 0;
      qreal left = 0;
      for (int end = finder.toNextBoundary(); end != -1; end = finder.toNextBoundary()) {
        const qreal right = end == region.text.size() ? 1.0 :
            qBound(left, totalWidth > 0 ? metrics.horizontalAdvance(region.text.left(end)) / totalWidth :
                                      qreal(end) / region.text.size(), 1.0);
        const qreal middle = (left + right) / 2;
        const QPointF top = region.box[0] + (region.box[1] - region.box[0]) * middle;
        const QPointF bottom = region.box[3] + (region.box[2] - region.box[3]) * middle;
        region.characters.append({begin, end, left, right, (top + bottom) / (2 * m_devicePixelRatio)});
        begin = end;
        left = right;
      }
      m_regions.append(region);
    }
    m_ocrReady = true;
    m_ocrVisible = true;
    rebuildTextBoxes();
    m_status = m_textBoxes.isEmpty() ? "No text found" :
        "Drag to select blue text. Ctrl+C copies the selection; Ctrl+Shift+C copies text inside the crop.";
  }
  emit ocrChanged();
}

bool CaptureEngine::rebuildTextBoxes() {
  QVariantList textBoxes;
  for (const auto &region : m_regions) {
    int start = 0;
    while (start < region.characters.size()) {
      const bool inside = m_selection.contains(region.characters[start].center);
      int end = start + 1;
      while (end < region.characters.size() &&
             m_selection.contains(region.characters[end].center) == inside)
        ++end;
      const auto &first = region.characters[start];
      const auto &last = region.characters[end - 1];
      const auto top = region.box[1] - region.box[0];
      const auto bottom = region.box[2] - region.box[3];
      const QPolygonF box{region.box[0] + top * first.left,
                          region.box[0] + top * last.right,
                          region.box[3] + bottom * last.right,
                          region.box[3] + bottom * first.left};
      QVariantList points;
      for (const auto &point : box)
        points.append(QVariant(QVariantList{point.x(), point.y()}));
      textBoxes.append(QVariantMap{
          {"text", region.text.mid(first.begin, last.end - first.begin)},
          {"box", points}, {"inside", inside}});
      start = end;
    }
  }
  if (textBoxes == m_textBoxes)
    return false;
  m_textBoxes = std::move(textBoxes);
  return true;
}

void CaptureEngine::copyText() {
  if (!m_ocrVisible)
    return;
  QStringList lines;
  for (const auto &box : m_textBoxes) {
    const auto run = box.toMap();
    if (run["inside"].toBool())
      lines.append(run["text"].toString());
  }
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
  const int left = qFloor(m_selection.left() * m_devicePixelRatio);
  const int top = qFloor(m_selection.top() * m_devicePixelRatio);
  const int right = qCeil(m_selection.right() * m_devicePixelRatio);
  const int bottom = qCeil(m_selection.bottom() * m_devicePixelRatio);
  QImage image = m_backgroundImage.copy(QRect(left, top, right - left, bottom - top)
                                           .intersected(m_backgroundImage.rect()))
                     .convertToFormat(QImage::Format_RGB888);
  image.setDevicePixelRatio(1.0);
  const QString savedPath = saveImage(image);
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
