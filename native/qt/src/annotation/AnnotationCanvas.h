#pragma once

#include "AnnotationDocument.h"
#include <QPointer>
#include <QQuickPaintedItem>

class AnnotationCanvas : public QQuickPaintedItem {
  Q_OBJECT
  Q_PROPERTY(AnnotationDocument *document READ document WRITE setDocument NOTIFY documentChanged)
  Q_PROPERTY(qreal pixelRatio READ pixelRatio WRITE setPixelRatio NOTIFY pixelRatioChanged)

public:
  explicit AnnotationCanvas(QQuickItem *parent = nullptr);
  AnnotationDocument *document() const { return m_document; }
  void setDocument(AnnotationDocument *document);
  qreal pixelRatio() const { return m_pixelRatio; }
  void setPixelRatio(qreal ratio);
  void paint(QPainter *painter) override;
  Q_INVOKABLE void setDialogOpen(bool open);

signals:
  void documentChanged();
  void pixelRatioChanged();
  void contextRequested(qreal x, qreal y, const QString &kind);

protected:
  void mousePressEvent(QMouseEvent *event) override;
  void mouseMoveEvent(QMouseEvent *event) override;
  void mouseReleaseEvent(QMouseEvent *event) override;
  void mouseDoubleClickEvent(QMouseEvent *event) override;
  void mouseUngrabEvent() override;
  void hoverMoveEvent(QHoverEvent *event) override;
  void hoverLeaveEvent(QHoverEvent *event) override;
  bool event(QEvent *event) override;

private:
  void refreshCursor();
  QPointer<AnnotationDocument> m_document;
  qreal m_pixelRatio = 1;
  QPointF m_hover;
  bool m_hovering = false;
  bool m_pressed = false;
  bool m_tablet = false;
};
