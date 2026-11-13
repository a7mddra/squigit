#pragma once

#include <QObject>
#include <QPointF>
#include <QPointer>
#include <QSet>
#include <Qt>

class QQuickItem;
class QQuickWindow;

class CaptureCursor : public QObject {
public:
  explicit CaptureCursor(QObject *parent = nullptr);
  ~CaptureCursor() override;
  void watch(QQuickWindow *window);
  void refresh();

protected:
  bool eventFilter(QObject *object, QEvent *event) override;

private:
  QQuickItem *itemAt(QQuickItem *item, QPointF position) const;
  void release();
  QSet<QQuickWindow *> m_windows;
  QPointer<QQuickWindow> m_window;
  QPointF m_position;
  Qt::MouseButtons m_buttons = Qt::NoButton;
  bool m_pending = false;
  bool m_overriding = false;
  int m_shape = -1;
  qint64 m_pixmapKey = 0;
};
