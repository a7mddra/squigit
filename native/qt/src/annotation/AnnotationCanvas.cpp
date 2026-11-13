#include "AnnotationCanvas.h"

#include <QPainter>

namespace {
bool brushCursor(const QString &tool) {
  return tool == "brush" || tool == "blur" || tool == "highlighter"
      || tool == "eraser" || tool == "objectEraser";
}
}

AnnotationCanvas::AnnotationCanvas(QQuickItem *parent) : QQuickPaintedItem(parent) {
  setAntialiasing(true);
  setOpaquePainting(false);
  setAcceptedMouseButtons(Qt::NoButton);
}

void AnnotationCanvas::setDocument(AnnotationDocument *document) {
  if (m_document == document) return;
  if (m_document) disconnect(m_document, nullptr, this, nullptr);
  m_document = document;
  if (document) {
    connect(document, &AnnotationDocument::changed, this, [this] {
      refreshCursor();
      const auto bounds = m_document->repaintBounds();
      if (bounds.isEmpty()) update();
      else update(QRectF(bounds.x() / m_pixelRatio, bounds.y() / m_pixelRatio,
                         bounds.width() / m_pixelRatio, bounds.height() / m_pixelRatio).adjusted(-3, -3, 3, 3).toAlignedRect());
    });
    connect(document, &AnnotationDocument::styleChanged, this, [this] { refreshCursor(); updateBrushCursor(); });
  }
  refreshCursor();
  updateBrushCursor();
  emit documentChanged();
  update();
}

void AnnotationCanvas::setPixelRatio(qreal ratio) {
  if (ratio <= 0 || ratio == m_pixelRatio) return;
  m_pixelRatio = ratio;
  refreshCursor();
  emit pixelRatioChanged();
  updateBrushCursor();
  update();
}

void AnnotationCanvas::paint(QPainter *p) {
  if (!m_document) return;
  p->setRenderHints(QPainter::Antialiasing | QPainter::SmoothPixmapTransform);
  p->save();
  p->scale(1 / m_pixelRatio, 1 / m_pixelRatio);
  m_document->paint(*p);
  p->restore();
  if (m_document->hasSelection()) {
    p->setPen(QPen(QColor("#ffffff"), 1));
    p->setBrush(Qt::NoBrush);
    for (auto polygon : m_document->selectedPolygons()) {
      for (auto &point : polygon) point /= m_pixelRatio;
      p->drawPolygon(polygon);
    }
    auto polygon = m_document->selectionPolygon();
    for (auto &point : polygon) point /= m_pixelRatio;
    if (m_document->selectionCount() > 1) p->drawPolygon(polygon);
    const auto handles = m_document->selectionHandles(m_pixelRatio);
    p->setBrush(QColor("#282828"));
    if (handles.size() > 8) p->drawLine((polygon[0] + polygon[1]) / 2, handles[8] / m_pixelRatio);
    for (int index = 0; index < handles.size(); ++index) {
      const auto point = handles[index] / m_pixelRatio;
      if (handles.size() == 2 || index >= 8) p->drawEllipse(point, index >= 9 ? 3 : 4, index >= 9 ? 3 : 4);
      else p->drawRect(QRectF(point - QPointF(3, 3), QSizeF(6, 6)));
    }
  }
  if (m_hovering) {
    const auto tool = m_document->tool();
    if (brushCursor(tool)) {
      const qreal radius = std::max(qreal(2.5), m_document->toolSize() / m_pixelRatio / 2);
      p->setBrush(Qt::NoBrush);
      p->setPen(QPen(QColor(0, 0, 0, 180), 2.5)); p->drawEllipse(m_hover, radius, radius);
      p->setPen(QPen(QColor(255, 255, 255, 230), 1)); p->drawEllipse(m_hover, radius, radius);
    }
  }
}

void AnnotationCanvas::refreshCursor() {
  int shape = Qt::ArrowCursor;
  if (m_document) {
    const auto tool = m_document->tool();
    const int handle = m_document->canTransformSelection()
        ? (m_pressed && m_dragHandle >= 0 ? m_dragHandle
            : m_document->selectionHandleAt(m_hover * m_pixelRatio, 8 * m_pixelRatio)) : -1;
    if (handle >= 0) {
      const auto handles = m_document->selectionHandles(m_pixelRatio);
      if (handle < handles.size()) {
        if (handles.size() == 2 || handle >= 8) shape = Qt::SizeAllCursor;
        else if (handle == 4 || handle == 6) shape = Qt::SizeVerCursor;
        else if (handle == 5 || handle == 7) shape = Qt::SizeHorCursor;
        else shape = handle == 0 || handle == 2 ? Qt::SizeFDiagCursor : Qt::SizeBDiagCursor;
      }
    } else if (brushCursor(tool)) shape = Qt::BlankCursor;
    else if (tool == "text") shape = Qt::IBeamCursor;
    else if (tool != "select") shape = Qt::CrossCursor;
  }
  if (shape == m_cursorShape) return;
  m_cursorShape = shape;
  emit cursorShapeChanged();
}

void AnnotationCanvas::hoverAt(QPointF point) {
  m_hover = point;
  m_hovering = boundingRect().contains(point);
  refreshCursor();
  updateBrushCursor();
}

void AnnotationCanvas::leave() {
  m_hovering = false;
  updateBrushCursor();
}

void AnnotationCanvas::updateBrushCursor() {
  QRectF bounds;
  if (m_document && m_hovering && brushCursor(m_document->tool())) {
    const qreal radius = std::max(qreal(2.5), m_document->toolSize() / m_pixelRatio / 2) + 3;
    bounds = QRectF(m_hover - QPointF(radius, radius), QSizeF(radius * 2, radius * 2));
  }
  const auto damage = m_brushCursorBounds.united(bounds);
  m_brushCursorBounds = bounds;
  if (!damage.isEmpty()) update(damage.toAlignedRect());
}

void AnnotationCanvas::pointerPress(QPointF point, int button, int modifiers) {
  if (!m_document) return;
  hoverAt(point);
  if (button == Qt::RightButton) {
    emit contextRequested(point.x(), point.y(), m_document->contextAt(point * m_pixelRatio, 6 * m_pixelRatio));
    return;
  }
  if (button != Qt::LeftButton) return;
  forceActiveFocus();
  m_pressed = true;
  m_dragHandle = m_document->selectionHandleAt(point * m_pixelRatio, 8 * m_pixelRatio);
  m_document->begin(point * m_pixelRatio, Qt::KeyboardModifiers(modifiers), 8 * m_pixelRatio);
  refreshCursor();
}

void AnnotationCanvas::pointerMove(QPointF point, int modifiers) {
  m_hover = point;
  m_hovering = boundingRect().contains(point);
  if (m_pressed && m_document) m_document->move(point * m_pixelRatio, Qt::KeyboardModifiers(modifiers));
  refreshCursor();
  updateBrushCursor();
}

void AnnotationCanvas::pointerRelease(QPointF point, int button, int modifiers) {
  if (m_pressed && m_document && button == Qt::LeftButton)
    m_document->finish(point * m_pixelRatio, Qt::KeyboardModifiers(modifiers));
  m_pressed = false;
  m_dragHandle = -1;
  hoverAt(point);
}

void AnnotationCanvas::pointerDoubleClick(QPointF point) {
  if (!m_document || (m_document->tool() != "select" && m_document->tool() != "text")) return;
  m_pressed = false;
  m_dragHandle = -1;
  m_document->editTextAt(point * m_pixelRatio, 6 * m_pixelRatio);
  refreshCursor();
}

void AnnotationCanvas::cancelPointer() {
  m_pressed = false;
  m_dragHandle = -1;
  if (m_document) m_document->cancelStroke();
  refreshCursor();
  update();
}
