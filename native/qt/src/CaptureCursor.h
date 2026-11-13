#pragma once

#include <QQuickItem>

class CaptureCursor : public QQuickItem {
  Q_OBJECT
  Q_PROPERTY(int cursorShape READ cursorShape WRITE setCursorShape NOTIFY cursorShapeChanged)

public:
  explicit CaptureCursor(QQuickItem *parent = nullptr);
  int cursorShape() const;
  void setCursorShape(int shape);
  Q_INVOKABLE void hoverAt(QPointF point);
  Q_INVOKABLE void refresh();

signals:
  void cursorShapeChanged();
  void hoverMoved(qreal x, qreal y);

protected:
  void hoverEnterEvent(QHoverEvent *event) override;
  void hoverMoveEvent(QHoverEvent *event) override;
  void hoverLeaveEvent(QHoverEvent *event) override;

private:
  QPointF m_position;
  bool m_hovering = false;
};
