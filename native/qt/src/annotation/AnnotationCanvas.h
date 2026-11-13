#pragma once

#include "AnnotationDocument.h"
#include <QPointer>
#include <QQuickPaintedItem>

class AnnotationCanvas : public QQuickPaintedItem {
  Q_OBJECT
  Q_PROPERTY(AnnotationDocument *document READ document WRITE setDocument NOTIFY documentChanged)
  Q_PROPERTY(qreal pixelRatio READ pixelRatio WRITE setPixelRatio NOTIFY pixelRatioChanged)
  Q_PROPERTY(int cursorShape READ cursorShape NOTIFY cursorShapeChanged)

public:
  explicit AnnotationCanvas(QQuickItem *parent = nullptr);
  AnnotationDocument *document() const { return m_document; }
  void setDocument(AnnotationDocument *document);
  qreal pixelRatio() const { return m_pixelRatio; }
  void setPixelRatio(qreal ratio);
  int cursorShape() const { return m_cursorShape; }
  void paint(QPainter *painter) override;
  Q_INVOKABLE void hoverAt(QPointF point);
  Q_INVOKABLE void leave();
  Q_INVOKABLE void pointerPress(QPointF point, int button, int modifiers);
  Q_INVOKABLE void pointerMove(QPointF point, int modifiers);
  Q_INVOKABLE void pointerRelease(QPointF point, int button, int modifiers);
  Q_INVOKABLE void pointerDoubleClick(QPointF point);
  Q_INVOKABLE void cancelPointer();

signals:
  void documentChanged();
  void pixelRatioChanged();
  void cursorShapeChanged();
  void contextRequested(qreal x, qreal y, const QString &kind);

private:
  void refreshCursor();
  void updateBrushCursor();
  QPointer<AnnotationDocument> m_document;
  qreal m_pixelRatio = 1;
  QPointF m_hover;
  QRectF m_brushCursorBounds;
  int m_cursorShape = Qt::ArrowCursor;
  int m_dragHandle = -1;
  bool m_hovering = false;
  bool m_pressed = false;
};
