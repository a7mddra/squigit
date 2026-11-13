#pragma once

#include <QPainterPath>
#include <QVector>

struct Annotation;
class QTextDocument;

namespace AnnotationGeometry {
QPainterPath outline(const Annotation &item, qreal extra = 0);
QPainterPath ink(const Annotation &item);
QPainterPath visibleInk(const Annotation &item);
QPainterPath textPath(const Annotation &item);
QVector<QPainterPath> connectedComponents(const QPainterPath &path);
void configureText(const Annotation &item, QTextDocument &document);
}
