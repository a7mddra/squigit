#include "CaptureCursor.h"

#include <QCursor>
#include <QHoverEvent>
#include <QQuickWindow>

CaptureCursor::CaptureCursor(QQuickItem *parent) : QQuickItem(parent) {
  setAcceptedMouseButtons(Qt::NoButton);
  setAcceptHoverEvents(true);
  setCursor(QCursor(Qt::ArrowCursor));
}

int CaptureCursor::cursorShape() const {
  return cursor().shape();
}

void CaptureCursor::setCursorShape(int shape) {
  const auto previous = cursor().shape();
  setCursor(QCursor(static_cast<Qt::CursorShape>(shape)));
  if (previous != shape) emit cursorShapeChanged();
}

void CaptureCursor::hoverAt(QPointF point) {
  m_position = point;
  m_hovering = boundingRect().contains(point);
  if (!isEnabled() || !isVisible()) return;
  emit hoverMoved(point.x(), point.y());
  if (m_hovering && window()) window()->setCursor(cursor());
}

void CaptureCursor::refresh() {
  if (m_hovering) hoverAt(m_position);
}

void CaptureCursor::hoverEnterEvent(QHoverEvent *event) {
  hoverAt(event->position());
  event->accept();
}

void CaptureCursor::hoverMoveEvent(QHoverEvent *event) {
  hoverAt(event->position());
  event->accept();
}

void CaptureCursor::hoverLeaveEvent(QHoverEvent *event) {
  m_hovering = false;
  setCursorShape(Qt::ArrowCursor);
  event->accept();
}
