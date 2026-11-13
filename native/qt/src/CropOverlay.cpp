#include "CropOverlay.h"

#include <QSGFlatColorMaterial>
#include <QSGGeometryNode>

CropOverlay::CropOverlay(QQuickItem *parent) : QQuickItem(parent) {
  setFlag(ItemHasContents, true);
  setAcceptedMouseButtons(Qt::NoButton);
}

void CropOverlay::setSelection(const QRectF &selection) {
  if (selection == m_selection)
    return;
  m_selection = selection;
  update();
  emit selectionChanged();
}

void CropOverlay::geometryChange(const QRectF &newGeometry,
                                const QRectF &oldGeometry) {
  QQuickItem::geometryChange(newGeometry, oldGeometry);
  if (newGeometry.size() != oldGeometry.size())
    update();
}

QSGNode *CropOverlay::updatePaintNode(QSGNode *oldNode, UpdatePaintNodeData *) {
  auto *node = static_cast<QSGGeometryNode *>(oldNode);
  if (!node) {
    node = new QSGGeometryNode();
    auto *geometry =
        new QSGGeometry(QSGGeometry::defaultAttributes_Point2D(), 16, 24);
    geometry->setDrawingMode(QSGGeometry::DrawTriangles);
    geometry->setVertexDataPattern(QSGGeometry::DynamicPattern);
    geometry->setIndexDataPattern(QSGGeometry::StaticPattern);
    auto *indices = geometry->indexDataAsUShort();
    for (int quad = 0; quad < 4; ++quad) {
      const unsigned short vertex = quad * 4;
      const int index = quad * 6;
      indices[index] = vertex;
      indices[index + 1] = vertex + 1;
      indices[index + 2] = vertex + 2;
      indices[index + 3] = vertex + 1;
      indices[index + 4] = vertex + 3;
      indices[index + 5] = vertex + 2;
    }
    geometry->markIndexDataDirty();
    node->setGeometry(geometry);
    node->setFlag(QSGNode::OwnsGeometry);
    auto *material = new QSGFlatColorMaterial();
    material->setColor(QColor(0, 0, 0, 112));
    node->setMaterial(material);
    node->setFlag(QSGNode::OwnsMaterial);
  }
  const auto bounds = boundingRect();
  const auto area = m_selection.normalized().intersected(bounds);
  const QRectF panels[] = {
      QRectF(0, 0, bounds.width(), area.top()),
      QRectF(0, area.bottom(), bounds.width(), bounds.height() - area.bottom()),
      QRectF(0, area.top(), area.left(), area.height()),
      QRectF(area.right(), area.top(), bounds.width() - area.right(), area.height())};
  auto *vertices = node->geometry()->vertexDataAsPoint2D();
  for (int quad = 0; quad < 4; ++quad) {
    const auto &panel = panels[quad];
    const int vertex = quad * 4;
    vertices[vertex].set(panel.left(), panel.top());
    vertices[vertex + 1].set(panel.right(), panel.top());
    vertices[vertex + 2].set(panel.left(), panel.bottom());
    vertices[vertex + 3].set(panel.right(), panel.bottom());
  }
  node->geometry()->markVertexDataDirty();
  node->markDirty(QSGNode::DirtyGeometry);
  return node;
}
