#include "AnnotationCanvas.h"

#include <QCursor>
#include <QHoverEvent>
#include <QMouseEvent>
#include <QPainter>
#include <QPixmap>
#include <QQuickWindow>
#include <QTabletEvent>
#include <cmath>
#include <iostream>

namespace {
bool brushCursor(const QString &tool) {
  return tool == "brush" || tool == "blur" || tool == "highlighter"
      || tool == "eraser" || tool == "objectEraser";
}

QCursor rotationCursor() {
  QPixmap image(64, 64);
  image.setDevicePixelRatio(2);
  image.fill(Qt::transparent);
  QPainter painter(&image);
  painter.setRenderHint(QPainter::Antialiasing);
  QPainterPath path;
  const QRectF circle(6, 6, 20, 20);
  path.arcMoveTo(circle, 45);
  path.arcTo(circle, 45, -285);
  const auto tip = path.currentPosition();
  const QPointF direction(.8660254037844386, -.5);
  const QPointF side(-direction.y(), direction.x());
  path.moveTo(tip - direction * 6 + side * 3);
  path.lineTo(tip);
  path.lineTo(tip - direction * 6 - side * 3);
  painter.setPen(QPen(QColor(0, 0, 0, 220), 4, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
  painter.drawPath(path);
  painter.setPen(QPen(Qt::white, 2, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
  painter.drawPath(path);
  painter.end();
  return QCursor(image, 16, 16);
}
}

AnnotationCanvas::AnnotationCanvas(QQuickItem *parent)
    : QQuickPaintedItem(parent), m_rotationCursor(rotationCursor()) {
  setAntialiasing(true);
  setOpaquePainting(false);
  setAcceptedMouseButtons(Qt::LeftButton | Qt::RightButton);
  setAcceptHoverEvents(true);
}

void AnnotationCanvas::setDocument(AnnotationDocument *document) {
  if (m_document == document) return;
  if (m_document) disconnect(m_document, nullptr, this, nullptr);
  m_document = document;
  if (document) {
    connect(document, &AnnotationDocument::changed, this, [this] { update(); });
    connect(document, &AnnotationDocument::styleChanged, this, [this] { refreshCursor(); update(); });
  }
  refreshCursor();
  emit documentChanged();
  update();
}

void AnnotationCanvas::setPixelRatio(qreal ratio) {
  if (ratio <= 0 || ratio == m_pixelRatio) return;
  m_pixelRatio = ratio;
  emit pixelRatioChanged();
  update();
}

void AnnotationCanvas::setDialogOpen(bool open) {
  if (!window() || (!open && !window()->isVisible())) return;
  if (!open) window()->requestActivate();
  if (!window()->setKeyboardGrabEnabled(!open))
    std::cerr << "CAPTURE_NATIVE_ERROR: could not transfer keyboard input for the image dialog" << std::endl;
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
    const auto handles = m_document->tool() == "select"
        ? m_document->selectionHandles(m_pixelRatio) : QVector<QPointF>{};
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
  if (!m_document) return;
  const auto tool = m_document->tool();
  if (brushCursor(tool)) {
    setCursor(Qt::BlankCursor);
    return;
  }
  const int handle = tool == "select" && m_document->hasSelection()
      ? (m_pressed && m_dragHandle >= 0 ? m_dragHandle
          : m_document->selectionHandleAt(m_hover * m_pixelRatio, 8 * m_pixelRatio)) : -1;
  if (handle >= 0) {
    const auto handles = m_document->selectionHandles(m_pixelRatio);
    if (handle >= handles.size()) {
      setCursor(Qt::ArrowCursor);
      return;
    }
    if (handle == 8) {
      setCursor(m_rotationCursor);
      return;
    }
    if (handle >= 9) setCursor(Qt::SizeAllCursor);
    else {
      const auto polygon = m_document->selectionPolygon();
      const auto center = (polygon[0] + polygon[2]) / 2;
      const auto direction = handles[handle] - center;
      const qreal angle = std::atan2(direction.y(), direction.x());
      const int sector = (int(std::round(angle / (3.14159265358979323846 / 4))) + 8) % 4;
      setCursor(sector == 0 ? Qt::SizeHorCursor : sector == 1 ? Qt::SizeFDiagCursor
          : sector == 2 ? Qt::SizeVerCursor : Qt::SizeBDiagCursor);
    }
  } else if (tool == "select" && m_pressed && m_document->hasSelection()) setCursor(Qt::ClosedHandCursor);
  else setCursor(tool == "select" ? Qt::ArrowCursor : tool == "text" ? Qt::IBeamCursor : Qt::CrossCursor);
}

void AnnotationCanvas::mousePressEvent(QMouseEvent *event) {
  if (!m_document || m_tablet) { event->ignore(); return; }
  m_hover = event->position();
  m_hovering = true;
  if (event->button() == Qt::RightButton) {
    const auto kind = m_document->contextAt(event->position() * m_pixelRatio, 6 * m_pixelRatio);
    emit contextRequested(event->position().x(), event->position().y(), kind);
  } else {
    forceActiveFocus();
    m_pressed = true;
    m_dragHandle = m_document->tool() == "select"
        ? m_document->selectionHandleAt(m_hover * m_pixelRatio, 8 * m_pixelRatio) : -1;
    m_document->begin(event->position() * m_pixelRatio, event->modifiers(), 8 * m_pixelRatio);
  }
  refreshCursor();
  event->accept();
}

void AnnotationCanvas::mouseMoveEvent(QMouseEvent *event) {
  m_hover = event->position(); m_hovering = true;
  if (m_pressed && m_document) m_document->move(event->position() * m_pixelRatio, event->modifiers());
  refreshCursor();
  update();
  event->accept();
}

void AnnotationCanvas::mouseReleaseEvent(QMouseEvent *event) {
  if (m_pressed && m_document && event->button() == Qt::LeftButton)
    m_document->finish(event->position() * m_pixelRatio, event->modifiers());
  m_pressed = false;
  m_dragHandle = -1;
  refreshCursor(); update(); event->accept();
}

void AnnotationCanvas::mouseDoubleClickEvent(QMouseEvent *event) {
  if (m_document && event->button() == Qt::LeftButton
      && (m_document->tool() == "select" || m_document->tool() == "text")) {
    m_pressed = false;
    m_dragHandle = -1;
    m_document->editTextAt(event->position() * m_pixelRatio, 6 * m_pixelRatio);
    event->accept();
  } else mousePressEvent(event);
}

void AnnotationCanvas::mouseUngrabEvent() {
  if (m_document) m_document->cancelStroke();
  m_pressed = false; m_tablet = false;
  m_dragHandle = -1;
  refreshCursor();
  update();
}

void AnnotationCanvas::hoverEnterEvent(QHoverEvent *event) {
  m_hover = event->position(); m_hovering = true;
  refreshCursor(); update();
}

void AnnotationCanvas::hoverMoveEvent(QHoverEvent *event) {
  m_hover = event->position(); m_hovering = true;
  refreshCursor(); update();
}

void AnnotationCanvas::hoverLeaveEvent(QHoverEvent *) { m_hovering = false; update(); }

bool AnnotationCanvas::event(QEvent *event) {
  if (event->type() == QEvent::TabletPress || event->type() == QEvent::TabletMove || event->type() == QEvent::TabletRelease) {
    if (!m_document) return false;
    auto *tablet = static_cast<QTabletEvent *>(event);
    const auto point = tablet->position() * m_pixelRatio;
    if (event->type() == QEvent::TabletPress) {
      m_tablet = true; forceActiveFocus();
      m_document->begin(point, tablet->modifiers(), 8 * m_pixelRatio);
    } else if (event->type() == QEvent::TabletMove && m_tablet) m_document->move(point, tablet->modifiers());
    else if (event->type() == QEvent::TabletRelease && m_tablet) {
      m_document->finish(point, tablet->modifiers()); m_tablet = false;
    }
    event->accept(); return true;
  }
  return QQuickPaintedItem::event(event);
}
