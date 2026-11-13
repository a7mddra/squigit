#include "CaptureEngine.h"
#include "Protocol.h"
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
#include <QTextBoundaryFinder>
#include <QTimer>
#include <QStandardPaths>
#include <QUuid>
#include <cmath>
#include <iostream>

CaptureEngine::CaptureEngine(QObject *parent) : QObject(parent),
    m_instanceId(QUuid::createUuid().toString(QUuid::WithoutBraces)) {
  m_qrTimer.setSingleShot(true);
  m_qrTimer.setInterval(250);
  connect(&m_qrTimer, &QTimer::timeout, this, &CaptureEngine::scanQr);
}

void CaptureEngine::setAction(const CaptureAction &action) {
  m_action = action;
  setCaptureMode(action.mode);
  setAutoOcr(action.autoOcr);
}

void CaptureEngine::retire() {
  m_retired = true;
  m_qrTimer.stop();
}

void CaptureEngine::setBackgroundImage(const QImage &image,
                                       qreal devicePixelRatio) {
  m_backgroundImage = image;
  if (m_action.id == "snapshot" || m_action.id == "ask-google")
    m_annotations.setImage(image);
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
  if (!m_retired)
    emit completed("cancelled", {});
}

void CaptureEngine::beginSelection() {
  if (m_captureStarted || m_retired)
    return;
  m_captureStarted = true;
  emit selectionStarted();
  if (m_autoOcr)
    QTimer::singleShot(0, this, [this]() { recognize(); });
}

void CaptureEngine::cancelOcr() {
  if (!m_ocrPending || m_ocrCancelling)
    return;
  m_ocrCancelling = true;
  m_status = "Stopping OCR...";
  emit ocrChanged();
  sendEvent({{"type", "ocr_cancel"}, {"id", m_requestId}});
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
  completeSelection();
}

void CaptureEngine::finishTraditionalCapture(QPointF start, QPointF end) {
  QRectF selectionRect = QRectF(start, end).normalized();

  if (selectionRect.width() < 1 || selectionRect.height() < 1) {
    fail();
    return;
  }

  select(selectionRect);
  completeSelection();
}

void CaptureEngine::completeSelection() {
  if (!m_retired && hasSelection() && m_action.selectionFinished)
    m_action.selectionFinished(this);
}

void CaptureEngine::select(const QRectF &logicalRect) {
  const QRectF bounds(0, 0, m_backgroundImage.width() / m_devicePixelRatio,
                      m_backgroundImage.height() / m_devicePixelRatio);
  const QRectF clipped = logicalRect.normalized().intersected(bounds);
  if (clipped.width() * m_devicePixelRatio < 1 ||
      clipped.height() * m_devicePixelRatio < 1 || clipped == m_selection)
    return;
  m_selection = clipped;
  if (!m_qrStatus.isEmpty() && !m_qrPending) {
    m_qrStatus.clear();
    emit qrChanged();
  }
  const bool textChanged = rebuildTextBoxes();
  emit selectionChanged();
  if (textChanged)
    emit ocrChanged();
}

void CaptureEngine::setSelection(const QRectF &logicalRect) {
  if (!m_retired && hasSelection()) {
    select(logicalRect);
    if (m_action.id == "scan-qr")
      m_qrTimer.start();
  }
}

void CaptureEngine::toggleOcr() {
  if (!hasSelection() || m_ocrPending)
    return;
  if (!m_qrPending) {
    m_qrStatus.clear();
    emit qrChanged();
  }
  if (!m_ocrReady) {
    recognize();
    return;
  }
  m_ocrVisible = !m_ocrVisible;
  emit ocrChanged();
}

void CaptureEngine::recognize() {
  if (m_retired || !m_captureStarted || m_ocrPending || m_ocrReady) return;
  m_ocrFailed = false;
  m_ocrFile = std::make_unique<QTemporaryFile>(QDir::temp().filePath("squigit-ocr-XXXXXX.png"));
  QImage image = m_backgroundImage.convertToFormat(QImage::Format_RGB888);
  image.setDevicePixelRatio(1.0);
  if (!m_ocrFile->open() || !image.save(m_ocrFile.get(), "PNG")) {
    m_ocrFile.reset();
    m_ocrFailed = true;
    m_status = "Could not prepare the display image. Retry OCR to scan again.";
    emit ocrChanged();
    return;
  }
  m_ocrFile->close();
  m_requestId = QString("ocr-%1-%2").arg(m_instanceId).arg(++m_requestSequence);
  m_ocrPending = true;
  m_status = "Recognizing display text...";
  emit ocrChanged();
  sendEvent({{"type", "ocr_request"}, {"id", m_requestId}, {"image", m_ocrFile->fileName()}});
}

void CaptureEngine::scanQr() {
  if (m_retired || !hasSelection() || m_qrPending)
    return;
  m_qrFile = std::make_unique<QTemporaryFile>(QDir::temp().filePath("squigit-qr-XXXXXX.png"));
  const QImage image = selectedImage();
  if (image.isNull() || !m_qrFile->open() || !image.save(m_qrFile.get(), "PNG")) {
    m_qrFile.reset();
    m_qrStatus = "Could not prepare the selected image. Resize the region to retry.";
    emit qrChanged();
    return;
  }
  m_qrFile->close();
  m_qrRequestId = QString("qr-%1-%2").arg(m_instanceId).arg(++m_qrRequestSequence);
  m_qrPending = true;
  m_qrStatus = "Scanning selected region for a QR code...";
  emit qrChanged();
  sendEvent({{"type", "qr_request"}, {"id", m_qrRequestId}, {"image", m_qrFile->fileName()}});
}

void CaptureEngine::acceptResponse(const QByteArray &line) {
  const QJsonObject response = QJsonDocument::fromJson(line).object();
  if (m_qrPending && response["id"].toString() == m_qrRequestId) {
    m_qrPending = false;
    m_qrFile.reset();
    const QString error = response["error"].toString();
    const QString url = response["url"].toString();
    if (!error.isEmpty() || url.isEmpty()) {
      m_qrStatus = error.isEmpty() ? "No QR code found in the selected region" : error;
      emit qrChanged();
      return;
    }
    emit completed("qr_open", url);
    return;
  }
  if (!m_ocrPending || response["id"].toString() != m_requestId) return;
  if (m_ocrCancelling && !response.contains("cancelled"))
    return;
  m_ocrPending = false;
  m_ocrCancelling = false;
  m_ocrFile.reset();
  if (response["cancelled"].toBool()) {
    m_ocrFailed = false;
    m_requestId.clear();
    m_status = "OCR cancelled. Scan OCR to start again.";
    emit ocrChanged();
    return;
  }
  const QString error = response["error"].toString();
  if (!error.isEmpty()) {
    m_ocrFailed = true;
    m_status = error + ". Retry OCR to scan again.";
  } else {
    m_ocrFailed = false;
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
  if (!m_ocrVisible || !hasSelection())
    return;
  QStringList lines;
  for (const auto &box : m_textBoxes) {
    const auto run = box.toMap();
    if (run["inside"].toBool())
      lines.append(run["text"].toString());
  }
  QGuiApplication::clipboard()->setText(lines.join("\n"));
}

QString CaptureEngine::saveImage(const QImage &rgb, const QString &directory) const {
  const QString outputDirectory = directory.isEmpty() ? m_action.outputDirectory : directory;
  if (outputDirectory.isEmpty())
    return {};
  const QDir base(outputDirectory);
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

QImage CaptureEngine::selectedImage(bool annotated) const {
  if (!hasSelection())
    return QImage();
  const int left = qFloor(m_selection.left() * m_devicePixelRatio);
  const int top = qFloor(m_selection.top() * m_devicePixelRatio);
  const int right = qCeil(m_selection.right() * m_devicePixelRatio);
  const int bottom = qCeil(m_selection.bottom() * m_devicePixelRatio);
  const QImage source = annotated ? m_annotations.flattened() : m_backgroundImage;
  QImage image = source.copy(QRect(left, top, right - left, bottom - top)
                                           .intersected(source.rect()))
                     .convertToFormat(QImage::Format_RGB888);
  image.setDevicePixelRatio(1.0);
  return image;
}

void CaptureEngine::save() {
  if (!hasSelection()) return;
  m_annotations.finishTextEditing();
  m_annotations.cancelStroke();
  const bool annotated = m_action.id == "snapshot" || m_action.id == "ask-google";
  const QString savedPath = saveImage(selectedImage(annotated));
  if (savedPath.isEmpty()) {
    std::cerr << "CAPTURE_NATIVE_ERROR: could not write the captured image"
              << std::endl;
    fail();
    return;
  }

  emit completed("saved", savedPath);
}

void CaptureEngine::copyImage(bool annotated) {
  if (m_retired || !hasSelection()) return;
  m_annotations.finishTextEditing();
  m_annotations.cancelStroke();
  QGuiApplication::clipboard()->setImage(selectedImage(annotated));
  m_annotations.setStatus(annotated ? "Annotated image copied" : "Original image copied");
}

void CaptureEngine::exportImage(bool annotated) {
  if (m_retired || !hasSelection()) return;
  m_annotations.finishTextEditing();
  m_annotations.cancelStroke();
  const auto directory = QDir(QStandardPaths::writableLocation(QStandardPaths::PicturesLocation)).filePath("squigit");
  const auto path = saveImage(selectedImage(annotated), directory);
  m_annotations.setStatus(path.isEmpty() ? "Could not export the image" : "Exported " + path);
}

void CaptureEngine::askGoogle(const QString &prompt) {
  if (m_retired || !hasSelection() || prompt.trimmed().isEmpty())
    return;
  m_annotations.finishTextEditing();
  m_annotations.cancelStroke();
  const bool annotated = m_action.id == "snapshot" || m_action.id == "ask-google";
  const QString savedPath = saveImage(selectedImage(annotated));
  if (savedPath.isEmpty()) {
    std::cerr << "CAPTURE_NATIVE_ERROR: could not write the captured image"
              << std::endl;
    fail();
    return;
  }

  emit askGoogleSubmitted(prompt, savedPath);
}

void CaptureEngine::fail() {
  if (!m_retired)
    emit completed("failed", "Could not complete the capture");
}
