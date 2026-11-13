#include "AnnotationDocument.h"
#include "StrokeSmoother.h"

#include <QAbstractTextDocumentLayout>
#include <QClipboard>
#include <QColorSpace>
#include <QDataStream>
#include <QFontMetricsF>
#include <QGuiApplication>
#include <QLineF>
#include <QMimeData>
#include <QPainter>
#include <QPainterPathStroker>
#include <QTextDocument>
#include <QTextOption>
#include <QtMath>
#include <algorithm>
#include <cmath>

namespace {
const QString objectMime = QStringLiteral("application/x-squigit-annotations");
bool freehand(const QString &tool) {
  return tool == "brush" || tool == "highlighter" || tool == "blur";
}
bool rectangle(const QString &tool) {
  return tool == "rectangle" || tool == "fillRectangle" || tool == "highlightRectangle";
}
QPainterPath outline(const Annotation &item, qreal extra = 0) {
  QPainterPathStroker stroker;
  stroker.setWidth(item.width + extra);
  stroker.setCapStyle(Qt::RoundCap);
  stroker.setJoinStyle(Qt::RoundJoin);
  return stroker.createStroke(item.path);
}
QPolygonF corners(const QRectF &rect) {
  return {rect.topLeft(), rect.topRight(), rect.bottomRight(), rect.bottomLeft()};
}
QTransform translation(QPointF delta) {
  return QTransform::fromTranslate(delta.x(), delta.y());
}
QImage blurPass(const QImage &source, int radius, bool horizontal) {
  QImage result(source.size(), QImage::Format_ARGB32_Premultiplied);
  const int major = horizontal ? source.height() : source.width();
  const int minor = horizontal ? source.width() : source.height();
  const int count = radius * 2 + 1;
  for (int row = 0; row < major; ++row) {
    auto pixel = [&](int col) {
      const int index = std::clamp(col, 0, minor - 1);
      return horizontal ? reinterpret_cast<const QRgb *>(source.constScanLine(row))[index]
                        : reinterpret_cast<const QRgb *>(source.constScanLine(index))[row];
    };
    int red = 0, green = 0, blue = 0, alpha = 0;
    auto accumulate = [&](QRgb value, int sign) {
      red += qRed(value) * sign;
      green += qGreen(value) * sign;
      blue += qBlue(value) * sign;
      alpha += qAlpha(value) * sign;
    };
    for (int col = -radius; col <= radius; ++col) accumulate(pixel(col), 1);
    for (int col = 0; col < minor; ++col) {
      const auto value = qRgba(red / count, green / count, blue / count, alpha / count);
      if (horizontal) reinterpret_cast<QRgb *>(result.scanLine(row))[col] = value;
      else reinterpret_cast<QRgb *>(result.scanLine(col))[row] = value;
      accumulate(pixel(col - radius), -1);
      accumulate(pixel(col + radius + 1), 1);
    }
  }
  return result;
}
}

AnnotationDocument::AnnotationDocument(QObject *parent) : QObject(parent) {
  m_history.append(QVector<Annotation>{});
  m_strokeUpdate.setSingleShot(true);
  m_strokeUpdate.setInterval(8);
  m_strokeUpdate.setTimerType(Qt::PreciseTimer);
  connect(&m_strokeUpdate, &QTimer::timeout, this, [this] {
    if (m_drawing) { updateActivePath(false); emit changed(); }
  });
}

void AnnotationDocument::setImage(const QImage &image) {
  finishTextEditing(false);
  cancelStroke();
  m_image = image.convertToFormat(QImage::Format_ARGB32_Premultiplied);
  m_image.setDevicePixelRatio(1);
  m_items.clear();
  m_history = {QVector<Annotation>{}};
  m_historyIndex = 0;
  m_selected.clear();
  notifySelection();
  emit imageChanged();
  emit historyChanged();
  emit changed();
}

void AnnotationDocument::setStatus(const QString &status) {
  if (status == m_status) return;
  m_status = status;
  emit statusChanged();
}

void AnnotationDocument::commit() {
  renumberCounters();
  m_history.resize(m_historyIndex + 1);
  m_history.append(m_items);
  ++m_historyIndex;
  emit historyChanged();
  emit changed();
}

void AnnotationDocument::notifySelection() {
  if (m_selected.size() == 1) {
    const auto &item = m_items[*m_selected.cbegin()];
    m_color = item.color;
    m_size = item.tool == "counter" ? item.rect.width() : item.width;
  }
  emit selectionChanged();
  emit textBoxChanged();
  emit styleChanged();
  emit changed();
}

bool AnnotationDocument::hasTextSelection() const {
  return m_selected.size() == 1 && m_items[*m_selected.cbegin()].tool == "text";
}

bool AnnotationDocument::canTransformSelection() const {
  if (!hasSelection()) return false;
  if (m_tool == "select") return true;
  if (m_selected.size() != 1) return false;
  const auto &item = m_items[*m_selected.cbegin()];
  const bool shape = rectangle(m_tool) || m_tool == "line" || m_tool == "arrow"
      || m_tool == "ellipse" || m_tool == "fillEllipse";
  return item.tool == m_tool && (shape || m_tool == "text");
}

bool AnnotationDocument::solidTool() const {
  const QString tool = m_selected.size() == 1 ? m_items[*m_selected.cbegin()].tool : m_tool;
  return tool.startsWith("fill") || tool == "highlightRectangle";
}

bool AnnotationDocument::colorTool() const {
  const QString tool = m_selected.size() == 1 ? m_items[*m_selected.cbegin()].tool : m_tool;
  return tool != "eraser" && tool != "objectEraser" && tool != "blur";
}

bool AnnotationDocument::mixedColors() const {
  if (m_selected.size() < 2) return false;
  const auto color = m_items[*m_selected.cbegin()].color;
  for (int index : m_selected) if (m_items[index].color != color) return true;
  return false;
}

void AnnotationDocument::setInkColor(const QColor &color) {
  if (!color.isValid()) return;
  m_color = color;
  bool modified = false;
  for (int index : m_selected) {
    auto &item = m_items[index];
    if (item.color != color) {
      item.color = color;
      modified = true;
    }
  }
  if (modified && !editingText()) {
    if (m_formatting) m_formatModified = true;
    else commit();
  }
  notifySelection();
}

void AnnotationDocument::setToolSize(qreal size) {
  m_size = std::clamp(size, qreal(1), qreal(160));
  m_sizes.insert(m_tool, m_size);
  bool modified = false;
  if (m_selected.size() == 1 && !solidTool()) {
    auto &item = m_items[*m_selected.cbegin()];
    if (item.tool == "counter") {
      const auto center = item.rect.center();
      item.rect = QRectF(center - QPointF(m_size / 2, m_size / 2), QSizeF(m_size, m_size));
    } else item.width = m_size;
    if (item.tool == "arrow") updateShape(item);
    modified = true;
  }
  if (modified && !editingText()) {
    if (m_formatting) m_formatModified = true;
    else commit();
  }
  emit styleChanged();
  emit textBoxChanged();
  emit changed();
}

void AnnotationDocument::beginFormatting() {
  m_formatting = true;
  m_formatModified = false;
}

void AnnotationDocument::finishFormatting() {
  m_formatting = false;
  if (m_formatModified && !editingText()) commit();
  m_formatModified = false;
}

void AnnotationDocument::selectTool(const QString &tool) {
  finishTextEditing();
  cancelStroke();
  m_sizes.insert(m_tool, m_size);
  m_tool = tool;
  const qreal defaultSize = tool == "text" ? 28 : tool == "counter" ? 40
      : tool == "blur" ? 48 : tool == "highlighter" || tool == "eraser" || tool == "objectEraser" ? 24 : 5;
  m_size = m_sizes.value(tool, defaultSize);
  m_selected.clear();
  notifySelection();
}

void AnnotationDocument::undo() {
  finishTextEditing();
  cancelStroke();
  if (!canUndo()) return;
  m_items = m_history[--m_historyIndex];
  m_selected.clear();
  notifySelection();
  emit historyChanged();
}

void AnnotationDocument::redo() {
  finishTextEditing();
  cancelStroke();
  if (!canRedo()) return;
  m_items = m_history[++m_historyIndex];
  m_selected.clear();
  notifySelection();
  emit historyChanged();
}

void AnnotationDocument::clear() {
  finishTextEditing();
  cancelStroke();
  if (m_items.isEmpty()) return;
  m_items.clear();
  m_selected.clear();
  notifySelection();
  commit();
}

void AnnotationDocument::deleteSelected() {
  finishTextEditing();
  cancelStroke();
  if (!hasSelection()) return;
  for (int index = m_items.size() - 1; index >= 0; --index)
    if (m_selected.contains(index)) m_items.removeAt(index);
  m_selected.clear();
  notifySelection();
  commit();
}

void AnnotationDocument::selectAll() {
  selectTool("select");
  m_selected.clear();
  for (int index = 0; index < m_items.size(); ++index) m_selected.insert(index);
  notifySelection();
}

void AnnotationDocument::deselect() {
  finishTextEditing();
  cancelStroke();
  m_selected.clear();
  notifySelection();
}

bool AnnotationDocument::handleEscape() {
  if (m_drawing || m_gesture != Gesture::None) { cancelStroke(); return true; }
  if (editingText()) { finishTextEditing(); return true; }
  if (hasSelection()) { deselect(); return true; }
  return false;
}

void AnnotationDocument::copySelected() {
  finishTextEditing();
  if (!hasSelection()) return;
  QByteArray data;
  QDataStream stream(&data, QIODevice::WriteOnly);
  stream.setVersion(QDataStream::Qt_6_4);
  stream << quint32(m_selected.size());
  for (int index = 0; index < m_items.size(); ++index) {
    if (!m_selected.contains(index)) continue;
    const auto &a = m_items[index];
    stream << a.tool << a.path << a.erased << a.color << a.width << a.opacity << a.rect
           << a.text << a.number << a.radius << a.autoHeight
           << a.start << a.end << a.blur << a.blurRect << a.transform;
  }
  auto *mime = new QMimeData;
  mime->setData(objectMime, data);
  QGuiApplication::clipboard()->setMimeData(mime);
  setStatus("Objects copied");
}

void AnnotationDocument::cutSelected() {
  copySelected();
  deleteSelected();
}

void AnnotationDocument::paste() {
  finishTextEditing();
  cancelStroke();
  const auto *mime = QGuiApplication::clipboard()->mimeData();
  if (!mime->hasFormat(objectMime)) { setStatus("Copy annotation objects to paste them here"); return; }
  const QByteArray data = mime->data(objectMime);
  QDataStream stream(data);
  stream.setVersion(QDataStream::Qt_6_4);
  quint32 count = 0;
  stream >> count;
  if (!count || count > 10000) return;
  QVector<Annotation> items;
  for (quint32 index = 0; index < count; ++index) {
    Annotation a;
    stream >> a.tool >> a.path >> a.erased >> a.color >> a.width >> a.opacity >> a.rect
           >> a.text >> a.number >> a.radius >> a.autoHeight
           >> a.start >> a.end >> a.blur >> a.blurRect >> a.transform;
    if (stream.status() != QDataStream::Ok) { setStatus("Could not paste the annotation objects"); return; }
    a.transform *= translation(QPointF(16, 16));
    items.append(a);
  }
  selectTool("select");
  for (const auto &item : items) { m_selected.insert(m_items.size()); m_items.append(item); }
  notifySelection();
  commit();
}

QRectF AnnotationDocument::localBounds(const Annotation &item) const {
  if (item.tool == "text" || item.tool == "counter" || rectangle(item.tool)
      || item.tool == "ellipse" || item.tool == "fillEllipse") return item.rect;
  return item.path.boundingRect().adjusted(-item.width / 2, -item.width / 2, item.width / 2, item.width / 2);
}

QRectF AnnotationDocument::selectionBounds() const {
  QRectF result;
  for (int index : m_selected) result = result.united(m_items[index].transform.mapRect(localBounds(m_items[index])));
  return result;
}

QPolygonF AnnotationDocument::selectionPolygon() const {
  if (!hasSelection()) return {};
  if (m_selected.size() == 1) {
    const auto &item = m_items[*m_selected.cbegin()];
    return item.transform.map(corners(localBounds(item)));
  }
  return corners(selectionBounds());
}

QVector<QPolygonF> AnnotationDocument::selectedPolygons() const {
  QVector<QPolygonF> result;
  for (int index : m_selected) result.append(m_items[index].transform.map(corners(localBounds(m_items[index]))));
  return result;
}

bool AnnotationDocument::lineSelection() const {
  if (m_selected.size() != 1) return false;
  const auto tool = m_items[*m_selected.cbegin()].tool;
  return tool == "line" || tool == "arrow";
}

bool AnnotationDocument::radiusSelection() const {
  return m_selected.size() == 1 && rectangle(m_items[*m_selected.cbegin()].tool);
}

QVector<QPointF> AnnotationDocument::selectionHandles(qreal pixelSize) const {
  if (!canTransformSelection()) return {};
  if (lineSelection()) {
    const auto &item = m_items[*m_selected.cbegin()];
    return {item.transform.map(item.start), item.transform.map(item.end)};
  }
  const auto polygon = selectionPolygon();
  QVector<QPointF> handles;
  for (const auto &point : polygon) handles.append(point);
  for (int i = 0; i < 4; ++i) handles.append((polygon[i] + polygon[(i + 1) % 4]) / 2);
  const auto top = handles[4], center = (polygon[0] + polygon[2]) / 2;
  const auto delta = top - center;
  handles.append(top + delta * (28 * pixelSize / std::max(qreal(.01), std::hypot(delta.x(), delta.y()))));
  if (radiusSelection()) {
    const auto &item = m_items[*m_selected.cbegin()];
    const auto rect = item.rect;
    const qreal inset = std::min({std::max(item.radius, 14 * pixelSize), rect.width() / 3, rect.height() / 3});
    for (const auto &point : corners(rect.adjusted(inset, inset, -inset, -inset))) handles.append(item.transform.map(point));
  }
  return handles;
}

int AnnotationDocument::selectionHandleAt(QPointF point, qreal tolerance) const {
  const auto handles = selectionHandles(tolerance / 8);
  for (int index = handles.size() - 1; index >= 0; --index)
    if (QLineF(point, handles[index]).length() <= tolerance) return index;
  return -1;
}

int AnnotationDocument::hitTest(QPointF point, qreal tolerance) const {
  for (int index = m_items.size() - 1; index >= 0; --index) {
    const auto &item = m_items[index];
    bool invertible = false;
    const auto inverse = item.transform.inverted(&invertible);
    if (!invertible) continue;
    const auto local = inverse.map(point);
    const qreal scale = std::max(qreal(.01), std::hypot(item.transform.m11(), item.transform.m12()));
    const qreal extra = tolerance / scale;
    if (item.erased.contains(local)) continue;
    if ((item.tool == "text" || item.tool == "counter")
        && item.rect.adjusted(-extra, -extra, extra, extra).contains(local)) return index;
    if ((item.tool.startsWith("fill") || item.tool == "highlightRectangle") && item.path.contains(local)) return index;
    if (outline(item, extra * 2).contains(local)) return index;
  }
  return -1;
}

QString AnnotationDocument::contextAt(QPointF point, qreal tolerance) {
  finishTextEditing();
  cancelStroke();
  const int index = hitTest(point, tolerance);
  if (m_tool != "select")
    return m_selected.size() > 1 && m_selected.contains(index) ? "object" : "format";
  if (index < 0) { m_selected.clear(); notifySelection(); return "canvas"; }
  if (!m_selected.contains(index)) { m_selected = {index}; notifySelection(); }
  return "object";
}

void AnnotationDocument::captureGesture() {
  m_beforeGesture = m_items;
  m_gesturePolygon = selectionPolygon();
}

void AnnotationDocument::begin(QPointF point, Qt::KeyboardModifiers modifiers, qreal tolerance) {
  cancelStroke();
  if (m_image.isNull()) return;
  m_start = m_last = point;
  m_pixelSize = tolerance / 8;
  const int handle = selectionHandleAt(point, tolerance);
  if (handle >= 0) {
    m_handle = handle;
    captureGesture();
    m_gesture = lineSelection() ? Gesture::Endpoint : handle == 8 ? Gesture::Rotate
        : handle >= 9 ? Gesture::Radius : Gesture::Resize;
    return;
  }
  finishTextEditing();
  if (m_tool == "eraser" || m_tool == "objectEraser") {
    m_selected.clear();
    captureGesture();
    m_gesture = Gesture::Erase;
    m_erased = false;
    eraseAt(point);
    notifySelection();
    return;
  }
  const int hit = m_tool == "select" ? hitTest(point, tolerance) : -1;
  if (m_tool == "select") {
    if (modifiers & Qt::ControlModifier) {
      if (m_selected.contains(hit)) { m_selected.remove(hit); notifySelection(); return; }
      if (hit >= 0) m_selected.insert(hit);
    } else if (!m_selected.contains(hit)) {
      m_selected.clear();
      if (hit >= 0) m_selected.insert(hit);
    }
    if (hit >= 0) { captureGesture(); m_gesture = Gesture::Move; }
    if (m_selected.size() == 1) {
      const auto &item = m_items[*m_selected.cbegin()];
      m_color = item.color;
      m_size = item.tool == "counter" ? item.rect.width() : item.width;
    }
    notifySelection();
    return;
  }
  m_selected.clear();
  if (m_tool == "text") {
    m_active = Annotation{};
    m_active.tool = "text";
    m_active.color = m_color;
    m_active.width = m_size;
    m_gesture = Gesture::CreateText;
    notifySelection();
    return;
  }
  m_active = Annotation{};
  m_active.tool = m_tool;
  m_active.color = m_color;
  m_active.width = m_size;
  m_active.start = m_active.end = point;
  m_active.opacity = m_tool == "highlighter" || m_tool == "highlightRectangle" ? .32 : 1;
  if (m_tool == "counter") {
    const qreal diameter = std::max(qreal(12), m_size);
    m_active.rect = QRectF(point - QPointF(diameter / 2, diameter / 2), QSizeF(diameter, diameter));
    m_active.number = nextCounter();
    m_items.append(m_active);
    commit();
    notifySelection();
    return;
  }
  if (m_tool == "blur") prepareBlur();
  m_drawing = true;
  m_points = {point};
  if (freehand(m_tool)) updateActivePath(false);
  else updateShape(m_active);
  notifySelection();
}

void AnnotationDocument::updateShape(Annotation &item) const {
  QPainterPath path;
  if (item.tool == "line" || item.tool == "arrow") {
    path.moveTo(item.start);
    path.lineTo(item.end);
    const auto delta = item.end - item.start;
    const qreal span = std::hypot(delta.x(), delta.y());
    if (item.tool == "arrow" && span > 1) {
      const auto direction = delta / span;
      const QPointF side(-direction.y(), direction.x());
      const qreal head = std::min(span * .45, std::max(qreal(14), item.width * 4));
      path.moveTo(item.end - direction * head + side * head * .5);
      path.lineTo(item.end);
      path.lineTo(item.end - direction * head - side * head * .5);
    }
  } else if (item.tool == "ellipse" || item.tool == "fillEllipse") path.addEllipse(item.rect);
  else path.addRoundedRect(item.rect, item.radius, item.radius);
  item.path = path;
}

void AnnotationDocument::updateActivePath(bool final) {
  const bool smooth = m_active.tool == "brush";
  const auto points = smooth ? StrokeSmoother::streamline(m_points, final ? .5 : .12) : m_points;
  if (smooth && final) m_active.path = StrokeSmoother::fit(points, .5, m_pixelSize);
  else {
    QPainterPath path;
    if (!points.isEmpty()) {
      path.moveTo(points.first());
      for (int index = 1; index < points.size(); ++index) path.lineTo(points[index]);
      if (points.size() == 1) path.lineTo(points.first() + QPointF(.001, 0));
    }
    m_active.path = path;
  }
}

void AnnotationDocument::move(QPointF point, Qt::KeyboardModifiers modifiers) {
  if (m_gesture == Gesture::Erase) { eraseAt(point); return; }
  if (m_gesture != Gesture::None) { transformSelection(point, modifiers); return; }
  if (!m_drawing) return;
  if (freehand(m_active.tool)) {
    if (QLineF(m_points.last(), point).length() < m_pixelSize * .25) return;
    m_points.append(point);
    if (!m_strokeUpdate.isActive()) m_strokeUpdate.start();
    return;
  }
  auto delta = point - m_start;
  if (modifiers & Qt::ShiftModifier) {
    if (m_tool == "line" || m_tool == "arrow") {
      const qreal angle = std::round(std::atan2(delta.y(), delta.x()) / (3.14159265358979323846 / 4)) * (3.14159265358979323846 / 4);
      const qreal length = std::hypot(delta.x(), delta.y());
      delta = QPointF(std::cos(angle) * length, std::sin(angle) * length);
    } else {
      const qreal side = std::max(std::abs(delta.x()), std::abs(delta.y()));
      delta = QPointF(std::copysign(side, delta.x()), std::copysign(side, delta.y()));
    }
  }
  m_active.end = m_start + delta;
  m_active.rect = modifiers & Qt::AltModifier ? QRectF(m_start - delta, m_start + delta).normalized()
                                           : QRectF(m_start, m_active.end).normalized();
  updateShape(m_active);
  emit changed();
}

void AnnotationDocument::transformSelection(QPointF point, Qt::KeyboardModifiers modifiers) {
  if (m_gesture == Gesture::CreateText) {
    if (!m_active.autoHeight || QLineF(point, m_start).length() > 4 * m_pixelSize) {
      const auto rect = QRectF(m_start, point).normalized();
      m_active.rect = rect;
      m_active.autoHeight = false;
    }
    emit changed();
    return;
  }
  if (!hasSelection()) return;
  if (m_gesture == Gesture::Endpoint) {
    const int index = *m_selected.cbegin();
    auto item = m_beforeGesture[index];
    const auto inverse = item.transform.inverted();
    const auto origin = m_handle == 0 ? item.start : item.end;
    auto local = origin + inverse.map(point) - inverse.map(m_start);
    if (modifiers & Qt::ShiftModifier) {
      const auto fixed = m_handle == 0 ? item.end : item.start;
      const auto delta = local - fixed;
      const qreal angle = std::round(qRadiansToDegrees(std::atan2(delta.y(), delta.x())) / 15) * 15;
      const qreal length = std::hypot(delta.x(), delta.y());
      local = fixed + QPointF(std::cos(qDegreesToRadians(angle)) * length, std::sin(qDegreesToRadians(angle)) * length);
    }
    if (m_handle == 0) item.start = local;
    else item.end = local;
    updateShape(item);
    m_items[index] = item;
  } else if (m_gesture == Gesture::Radius) {
    const int index = *m_selected.cbegin();
    auto item = m_beforeGesture[index];
    const auto inverse = item.transform.inverted();
    const auto delta = inverse.map(point) - inverse.map(m_start);
    const int corner = m_handle - 9;
    const qreal inwardX = (corner == 0 || corner == 3) ? delta.x() : -delta.x();
    const qreal inwardY = (corner == 0 || corner == 1) ? delta.y() : -delta.y();
    const qreal movement = qAbs(inwardX) > qAbs(inwardY) ? inwardX : inwardY;
    item.radius = std::clamp(item.radius + movement, qreal(0), std::min(item.rect.width(), item.rect.height()) / 2);
    updateShape(item);
    m_items[index] = item;
  } else {
    QTransform transform;
    if (m_gesture == Gesture::Move) transform = translation(point - m_start);
    else if (m_gesture == Gesture::Rotate) {
      const auto center = (m_gesturePolygon[0] + m_gesturePolygon[2]) / 2;
      const auto start = m_start - center, current = point - center;
      qreal angle = qRadiansToDegrees(std::atan2(current.y(), current.x()) - std::atan2(start.y(), start.x()));
      if (modifiers & Qt::ShiftModifier) angle = std::round(angle / 15) * 15;
      transform.translate(center.x(), center.y());
      transform.rotate(angle);
      transform.translate(-center.x(), -center.y());
    } else if (m_gesture == Gesture::Resize) {
      const auto &polygon = m_gesturePolygon;
      QTransform frame(polygon[1].x() - polygon[0].x(), polygon[1].y() - polygon[0].y(),
                       polygon[3].x() - polygon[0].x(), polygon[3].y() - polygon[0].y(),
                       polygon[0].x(), polygon[0].y());
      bool invertible = false;
      const auto inverse = frame.inverted(&invertible);
      if (!invertible) return;
      const QVector<QPointF> handles{{0, 0}, {1, 0}, {1, 1}, {0, 1}, {.5, 0}, {1, .5}, {.5, 1}, {0, .5}};
      const auto local = handles[m_handle] + inverse.map(point) - inverse.map(m_start);
      qreal left = 0, top = 0, right = 1, bottom = 1;
      const qreal minimumX = 4 * m_pixelSize / std::max(qreal(1), QLineF(polygon[0], polygon[1]).length());
      const qreal minimumY = 4 * m_pixelSize / std::max(qreal(1), QLineF(polygon[0], polygon[3]).length());
      if (m_handle == 0 || m_handle == 3 || m_handle == 7) left = std::min(local.x(), 1 - minimumX);
      if (m_handle == 1 || m_handle == 2 || m_handle == 5) right = std::max(local.x(), minimumX);
      if (m_handle == 0 || m_handle == 1 || m_handle == 4) top = std::min(local.y(), 1 - minimumY);
      if (m_handle == 2 || m_handle == 3 || m_handle == 6) bottom = std::max(local.y(), minimumY);
      if ((modifiers & Qt::ShiftModifier) && m_handle < 4) {
        const qreal scale = std::max(right - left, bottom - top);
        if (m_handle == 0 || m_handle == 3) left = right - scale; else right = left + scale;
        if (m_handle == 0 || m_handle == 1) top = bottom - scale; else bottom = top + scale;
      }
      QTransform scaling;
      scaling.translate(left, top);
      scaling.scale(right - left, bottom - top);
      transform = inverse * scaling * frame;
    }
    for (int index : m_selected) m_items[index].transform = m_beforeGesture[index].transform * transform;
  }
  emit textBoxChanged();
  emit changed();
}

void AnnotationDocument::finish(QPointF point, Qt::KeyboardModifiers modifiers) {
  if (m_gesture != Gesture::None) {
    move(point, modifiers);
    const auto gesture = m_gesture;
    m_gesture = Gesture::None;
    if (gesture == Gesture::CreateText) {
      QFont font = QGuiApplication::font();
      font.setPixelSize(std::max(1, qRound(m_active.width)));
      const qreal height = qCeil(QFontMetricsF(font).height()) + 12;
      if (m_active.autoHeight) {
        m_active.rect = QRectF(m_start, QSizeF(std::min(qreal(280), std::max(qreal(80), m_image.width() - m_start.x())), height));
      } else {
        m_active.rect.setWidth(std::max(qreal(36), m_active.rect.width()));
        m_active.rect.setHeight(std::max(height, m_active.rect.height()));
      }
      m_items.append(m_active);
      m_active = Annotation{};
      beginTextEditing(m_items.size() - 1, true);
      return;
    }
    if (!editingText()
        && (gesture == Gesture::Erase ? m_erased : point != m_start)) commit();
    m_beforeGesture.clear();
    notifySelection();
    return;
  }
  if (!m_drawing) return;
  move(point, modifiers);
  m_strokeUpdate.stop();
  m_drawing = false;
  if (freehand(m_active.tool)) {
    if (point != m_points.last()) m_points.append(point);
    updateActivePath(true);
    if (m_active.tool == "blur") cropBlur();
  } else if (QLineF(m_start, point).length() < 1
      || ((m_active.tool != "line" && m_active.tool != "arrow")
          && (m_active.rect.width() < 1 || m_active.rect.height() < 1))) {
    m_active = Annotation{};
    emit changed();
    return;
  }
  const bool shape = !freehand(m_active.tool);
  m_items.append(m_active);
  if (shape) m_selected = {int(m_items.size()) - 1};
  m_active = Annotation{};
  m_points.clear();
  if (shape) notifySelection();
  commit();
}

void AnnotationDocument::cancelStroke() {
  m_strokeUpdate.stop();
  const auto gesture = m_gesture;
  const bool active = m_drawing || gesture != Gesture::None;
  m_gesture = Gesture::None;
  if (gesture != Gesture::None && !m_beforeGesture.isEmpty()) m_items = m_beforeGesture;
  m_beforeGesture.clear();
  m_drawing = false;
  m_active = Annotation{};
  m_points.clear();
  if (active) { emit textBoxChanged(); emit changed(); }
}

void AnnotationDocument::eraseAt(QPointF point) {
  QPainterPath segment;
  segment.moveTo(m_last);
  segment.lineTo(point == m_last ? point + QPointF(.001, 0) : point);
  m_last = point;
  QPainterPathStroker stroker;
  stroker.setWidth(m_size);
  stroker.setCapStyle(Qt::RoundCap);
  const auto swept = stroker.createStroke(segment);
  bool modified = false;
  for (int index = m_items.size() - 1; index >= 0; --index) {
    auto &item = m_items[index];
    const auto local = item.transform.inverted().map(swept);
    const bool shapeFill = item.tool.startsWith("fill") || item.tool == "highlightRectangle";
    QPainterPath ink;
    if (item.tool == "text" || item.tool == "counter") ink.addRect(item.rect);
    else ink = shapeFill ? item.path : outline(item);
    const auto remaining = ink.subtracted(item.erased);
    if (!remaining.intersects(local)) continue;
    if (m_tool == "objectEraser") { m_items.removeAt(index); modified = true; continue; }
    item.erased = item.erased.united(local);
    if (ink.subtracted(item.erased).isEmpty()) m_items.removeAt(index);
    modified = true;
  }
  m_erased = m_erased || modified;
  if (modified) { renumberCounters(); emit changed(); }
}

void AnnotationDocument::renumberCounters() {
  int number = 0;
  for (auto &item : m_items)
    if (item.tool == "counter") item.number = ++number;
}

int AnnotationDocument::nextCounter() const {
  int next = 1;
  for (const auto &item : m_items) if (item.tool == "counter") next = std::max(next, item.number + 1);
  return next;
}

void AnnotationDocument::prepareBlur() {
  const qreal scale = std::min(qreal(.5), 1200.0 / std::max(m_image.width(), m_image.height()));
  QImage image(QSize(std::max(1, qRound(m_image.width() * scale)), std::max(1, qRound(m_image.height() * scale))),
               QImage::Format_ARGB32_Premultiplied);
  image.fill(Qt::transparent);
  { QPainter painter(&image); painter.scale(qreal(image.width()) / m_image.width(), qreal(image.height()) / m_image.height()); paint(painter, false); }
  const int radius = std::max(2, qRound(12 * scale));
  for (int pass = 0; pass < 3; ++pass) { image = blurPass(image, radius, true); image = blurPass(image, radius, false); }
  m_active.blur = image;
  m_active.blurRect = QRectF(QPointF(), m_image.size());
}

void AnnotationDocument::cropBlur() {
  const qreal sx = qreal(m_active.blur.width()) / m_image.width();
  const qreal sy = qreal(m_active.blur.height()) / m_image.height();
  const auto area = localBounds(m_active).adjusted(-4 / sx, -4 / sy, 4 / sx, 4 / sy).intersected(QRectF(QPointF(), m_image.size()));
  const auto pixels = QRectF(area.x() * sx, area.y() * sy, area.width() * sx, area.height() * sy).toAlignedRect().intersected(m_active.blur.rect());
  m_active.blur = m_active.blur.copy(pixels);
  m_active.blurRect = QRectF(pixels.x() / sx, pixels.y() / sy, pixels.width() / sx, pixels.height() / sy);
}

QVariantMap AnnotationDocument::textBox() const {
  if (!editingText()) return {};
  const auto &item = m_items[m_textIndex];
  const auto &t = item.transform;
  return {{"x", item.rect.x()}, {"y", item.rect.y()}, {"width", item.rect.width()}, {"height", item.rect.height()},
          {"text", item.text}, {"fontSize", item.width}, {"color", item.color},
          {"m11", t.m11()}, {"m12", t.m12()}, {"m21", t.m21()}, {"m22", t.m22()}, {"dx", t.dx()}, {"dy", t.dy()}};
}

void AnnotationDocument::beginTextEditing(int index, bool draft) {
  finishTextEditing();
  m_sizes.insert(m_tool, m_size);
  m_tool = "select";
  m_selected = {index};
  m_textIndex = index;
  m_textDraft = draft;
  m_textOriginal = m_items[index];
  m_color = m_items[index].color;
  m_size = m_items[index].width;
  notifySelection();
}

void AnnotationDocument::editTextAt(QPointF point, qreal tolerance) {
  finishTextEditing();
  cancelStroke();
  const int index = hitTest(point, tolerance);
  if (index >= 0 && m_items[index].tool == "text") beginTextEditing(index);
}

void AnnotationDocument::updateText(const QString &text) {
  if (!editingText() || m_items[m_textIndex].text == text) return;
  m_items[m_textIndex].text = text;
  emit textBoxChanged(); emit changed();
}

void AnnotationDocument::setTextContentHeight(qreal height) {
  if (!editingText()) return;
  auto &item = m_items[m_textIndex];
  if (!item.autoHeight || height < 12 || qAbs(height - item.rect.height()) <= .5) return;
  item.rect.setHeight(height);
  emit textBoxChanged(); emit changed();
}

void AnnotationDocument::finishTextEditing(bool accept) {
  if (!editingText()) return;
  const int index = m_textIndex;
  m_textIndex = -1;
  const auto &item = m_items[index];
  const bool modified = m_textDraft || item.text != m_textOriginal.text || item.color != m_textOriginal.color
      || item.width != m_textOriginal.width || item.rect != m_textOriginal.rect
      || item.transform != m_textOriginal.transform;
  if (!accept) {
    if (m_textDraft) { m_items.removeAt(index); m_selected.clear(); }
    else m_items[index] = m_textOriginal;
  } else if (item.text.trimmed().isEmpty()) {
    m_items.removeAt(index); m_selected.clear();
    if (!m_textDraft) commit();
  } else if (modified) commit();
  m_textDraft = false;
  notifySelection();
}

void AnnotationDocument::paintAnnotation(QPainter &p, const Annotation &item) const {
  p.save();
  p.setWorldTransform(item.transform, true);
  p.setOpacity(item.opacity);
  if (!item.erased.isEmpty()) {
    QPainterPath visible;
    visible.addRect(localBounds(item).adjusted(-item.width, -item.width, item.width, item.width));
    p.setClipPath(visible.subtracted(item.erased), Qt::IntersectClip);
  }
  if (item.tool == "blur") {
    p.setClipPath(outline(item), Qt::IntersectClip);
    p.drawImage(item.blurRect, item.blur);
  } else if (item.tool == "counter") {
    p.setPen(QPen(Qt::white, 2)); p.setBrush(item.color); p.drawEllipse(item.rect);
    QFont font = QGuiApplication::font();
    font.setPixelSize(std::max(1, qRound(item.rect.height() * .45))); font.setWeight(QFont::Bold);
    p.setFont(font);
    const qreal luminance = .2126 * item.color.redF() + .7152 * item.color.greenF() + .0722 * item.color.blueF();
    p.setPen(luminance > .6 ? QColor("#171717") : Qt::white);
    p.drawText(item.rect, Qt::AlignCenter, QString::number(item.number));
  } else if (item.tool == "text") {
    QFont font = QGuiApplication::font(); font.setPixelSize(std::max(1, qRound(item.width)));
    QTextDocument text;
    text.setDefaultFont(font); text.setDocumentMargin(0);
    QTextOption option; option.setWrapMode(QTextOption::WrapAtWordBoundaryOrAnywhere); text.setDefaultTextOption(option);
    text.setPlainText(item.text);
    const auto inner = item.rect.adjusted(6, 6, -6, -6);
    text.setTextWidth(inner.width());
    p.setClipRect(inner, Qt::IntersectClip); p.translate(inner.topLeft());
    QAbstractTextDocumentLayout::PaintContext context; context.palette.setColor(QPalette::Text, item.color);
    text.documentLayout()->draw(&p, context);
  } else {
    const bool filled = item.tool.startsWith("fill") || item.tool == "highlightRectangle";
    p.setPen(filled ? QPen(Qt::NoPen) : QPen(item.color, item.width, Qt::SolidLine,
               item.tool == "highlighter" ? Qt::FlatCap : Qt::RoundCap, Qt::RoundJoin));
    p.setBrush(filled ? QBrush(item.color) : Qt::NoBrush);
    p.drawPath(item.path);
  }
  p.restore();
}

void AnnotationDocument::paint(QPainter &p, bool interactive) const {
  p.setRenderHints(QPainter::Antialiasing | QPainter::SmoothPixmapTransform | QPainter::TextAntialiasing);
  p.drawImage(QPointF(), m_image);
  for (int index = 0; index < m_items.size(); ++index) {
    if (interactive && m_textIndex == index) continue;
    paintAnnotation(p, m_items[index]);
  }
  if (interactive && m_drawing) paintAnnotation(p, m_active);
  if (interactive && m_gesture == Gesture::CreateText && !m_active.autoHeight) {
    p.save();
    p.setPen(QPen(Qt::white, m_pixelSize, Qt::DashLine));
    p.setBrush(Qt::NoBrush);
    p.drawRect(m_active.rect);
    p.restore();
  }
}

QImage AnnotationDocument::flattened() const {
  QImage image(m_image.size(), QImage::Format_ARGB32_Premultiplied);
  image.setColorSpace(m_image.colorSpace());
  image.fill(Qt::transparent);
  QPainter painter(&image);
  paint(painter, false);
  return image;
}
