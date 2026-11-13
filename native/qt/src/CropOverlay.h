#pragma once

#include <QQuickItem>
#include <QRectF>

class CropOverlay : public QQuickItem {
  Q_OBJECT
  Q_PROPERTY(QRectF selection READ selection WRITE setSelection NOTIFY selectionChanged)

public:
  explicit CropOverlay(QQuickItem *parent = nullptr);
  QRectF selection() const { return m_selection; }
  void setSelection(const QRectF &selection);

signals:
  void selectionChanged();

protected:
  QSGNode *updatePaintNode(QSGNode *oldNode, UpdatePaintNodeData *) override;
  void geometryChange(const QRectF &newGeometry,
                      const QRectF &oldGeometry) override;

private:
  QRectF m_selection;
};
