#pragma once

#include <QPainterPath>
#include <QVector>

namespace StrokeSmoother {

QVector<QPointF> streamline(const QVector<QPointF> &points, qreal factor);

QPainterPath fit(const QVector<QPointF> &points, qreal smoothing, qreal pixelSize);

}
