#include "CaptureCursor.h"

#include <QCursor>
#include <QEvent>
#include <QGuiApplication>
#include <QMouseEvent>
#include <QPixmap>
#include <QQuickItem>
#include <QQuickWindow>
#include <QTimer>
#include <QVariant>
#include <algorithm>

CaptureCursor::CaptureCursor(QObject *parent) : QObject(parent) {}

CaptureCursor::~CaptureCursor() { release(); }

void CaptureCursor::watch(QQuickWindow *window) {
  m_windows.insert(window);
  window->installEventFilter(this);
  connect(window, &QObject::destroyed, this, [this, window] {
    m_windows.remove(window);
    if (!m_window || m_window == window) {
      m_window.clear();
      release();
    }
  });
}

void CaptureCursor::release() {
  if (!m_overriding) return;
  QGuiApplication::restoreOverrideCursor();
  m_overriding = false;
  m_shape = -1;
  m_pixmapKey = 0;
}

QQuickItem *CaptureCursor::itemAt(QQuickItem *item, QPointF position) const {
  if (!item->isVisible() || !item->isEnabled()) return nullptr;
  const bool contains = item->contains(item->mapFromScene(position));
  if (item->clip() && !contains) return nullptr;
  auto children = item->childItems();
  std::stable_sort(children.begin(), children.end(), [](QQuickItem *left, QQuickItem *right) {
    return left->z() < right->z();
  });
  for (auto child = children.crbegin(); child != children.crend(); ++child)
    if (auto *target = itemAt(*child, position)) return target;
  return contains && (item->acceptedMouseButtons() != Qt::NoButton || item->acceptHoverEvents())
      ? item : nullptr;
}

void CaptureCursor::refresh() {
  if (m_pending) return;
  m_pending = true;
  QTimer::singleShot(0, this, [this] {
    m_pending = false;
    if (!m_window || !m_window->isVisible() || m_window->property("nativeDialogOpen").toBool()) {
      release();
      return;
    }
    QQuickItem *target = m_buttons != Qt::NoButton ? m_window->mouseGrabberItem() : nullptr;
    if (!target) target = itemAt(m_window->contentItem(), m_position);
    const QCursor cursor = target ? target->cursor() : QCursor(Qt::ArrowCursor);
    const auto pixmapKey = cursor.pixmap().cacheKey();
    if (!m_overriding) {
      QGuiApplication::setOverrideCursor(cursor);
      m_overriding = true;
    } else if (m_shape != cursor.shape() || m_pixmapKey != pixmapKey) {
      QGuiApplication::changeOverrideCursor(cursor);
    }
    m_shape = cursor.shape();
    m_pixmapKey = pixmapKey;
  });
}

bool CaptureCursor::eventFilter(QObject *object, QEvent *event) {
  auto *window = qobject_cast<QQuickWindow *>(object);
  if (!window || !m_windows.contains(window)) return false;
  const auto type = event->type();
  if (type == QEvent::Leave || type == QEvent::Hide || type == QEvent::WindowDeactivate) {
    if (m_window == window) {
      m_window.clear();
      m_buttons = Qt::NoButton;
      release();
    }
  } else if (type == QEvent::Enter) {
    m_window = window;
    m_position = window->mapFromGlobal(QCursor::pos());
    m_buttons = QGuiApplication::mouseButtons();
    refresh();
  } else if (type == QEvent::MouseMove || type == QEvent::MouseButtonPress
             || type == QEvent::MouseButtonRelease || type == QEvent::MouseButtonDblClick) {
    auto *mouse = static_cast<QMouseEvent *>(event);
    m_window = window;
    m_position = mouse->position();
    m_buttons = mouse->buttons();
    refresh();
  }
  return false;
}
