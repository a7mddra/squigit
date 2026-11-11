#include "RadialPetal.h"
#include "RadialGeometry.h"

#include <QPainter>
#include <QPainterPathStroker>
#include <QPen>
#include <QTransform>
#include <algorithm>
#include <cmath>

namespace {
constexpr qreal Pi = 3.14159265358979323846;

QPointF mirrored(const QPointF &point) {
    return QPointF(-point.x(), point.y());
}

QPointF areaCenter(const QPainterPath &path) {
    const QPolygonF polygon = path.toFillPolygon();
    qreal area = 0;
    QPointF weightedCenter;
    for (int index = 0; index < polygon.size(); ++index) {
        const QPointF &from = polygon[index];
        const QPointF &to = polygon[(index + 1) % polygon.size()];
        const qreal cross = from.x() * to.y() - to.x() * from.y();
        area += cross;
        weightedCenter += (from + to) * cross;
    }
    return weightedCenter / (3 * area);
}

void arcTo(QPainterPath &path, const QPointF &center, qreal radius,
           const QPointF &from, const QPointF &to, bool clockwise = true) {
    const auto angle = [&center](const QPointF &point) {
        return std::atan2(center.y() - point.y(), point.x() - center.x()) * 180 / Pi;
    };
    const qreal start = angle(from);
    qreal sweep = angle(to) - start;
    if (clockwise && sweep > 0) sweep -= 360;
    if (!clockwise && sweep < 0) sweep += 360;
    path.arcTo(QRectF(center.x() - radius, center.y() - radius, radius * 2, radius * 2),
               start, sweep);
}

QPainterPath petalPath(qreal outer) {
    using namespace RadialGeometry;
    const qreal halfAngle = Pi / 5;
    const QPointF direction(std::sin(halfAngle), -std::cos(halfAngle));
    const QPointF inward(-std::cos(halfAngle), -std::sin(halfAngle));
    const qreal inner = HoleRadius + InnerRim;
    const qreal inset = Gap / 2 + CornerRadius;

    const auto cornerCenter = [&](qreal radius) {
        return direction * std::sqrt(radius * radius - inset * inset) + inward * inset;
    };
    const QPointF outerCorner = cornerCenter(outer - CornerRadius);
    const QPointF innerCorner = cornerCenter(inner + CornerRadius);
    const QPointF outerArc = outerCorner * (outer / (outer - CornerRadius));
    const QPointF innerArc = innerCorner * (inner / (inner + CornerRadius));
    const QPointF outerSide = outerCorner - inward * CornerRadius;
    const QPointF innerSide = innerCorner - inward * CornerRadius;

    QPainterPath path;
    path.moveTo(mirrored(outerArc));
    arcTo(path, QPointF(), outer, mirrored(outerArc), outerArc);
    arcTo(path, outerCorner, CornerRadius, outerArc, outerSide);
    path.lineTo(innerSide);
    arcTo(path, innerCorner, CornerRadius, innerSide, innerArc);
    arcTo(path, QPointF(), inner, innerArc, mirrored(innerArc), false);
    arcTo(path, mirrored(innerCorner), CornerRadius, mirrored(innerArc), mirrored(innerSide));
    path.lineTo(mirrored(outerSide));
    arcTo(path, mirrored(outerCorner), CornerRadius, mirrored(outerSide), mirrored(outerArc));
    path.closeSubpath();
    path.translate(OuterRadius, OuterRadius);
    return path;
}
}

RadialPetal::RadialPetal(QQuickItem *parent)
    : QQuickPaintedItem(parent),
      outerRadius_(RadialGeometry::OuterRadius - RadialGeometry::OuterRim) {
    setAntialiasing(true);
    path_ = petalPath(outerRadius_);
    basePath_ = path_;
    revealPath_ = path_;
    shadowPath_ = revealPath_.translated(0, 6);
}

QColor RadialPetal::fillColor() const {
    return fillColor_;
}

void RadialPetal::setFillColor(const QColor &color) {
    if (fillColor_ == color) return;
    fillColor_ = color;
    emit fillColorChanged();
    update();
}

qreal RadialPetal::angle() const {
    return angle_;
}

void RadialPetal::setAngle(qreal angle) {
    if (angle_ == angle) return;
    angle_ = angle;
    orientation_.reset();
    orientation_.translate(RadialGeometry::OuterRadius, RadialGeometry::OuterRadius);
    orientation_.rotate(angle);
    orientation_.translate(-RadialGeometry::OuterRadius, -RadialGeometry::OuterRadius);
    path_ = orientation_.map(basePath_);
    revealPath_ = orientation_.map(petalPath(outerRadius_));
    shadowPath_ = revealPath_.translated(0, 6);
    emit angleChanged();
    updateContentRect();
    update();
}

qreal RadialPetal::outerRadius() const {
    return outerRadius_;
}

void RadialPetal::setOuterRadius(qreal radius) {
    using namespace RadialGeometry;
    radius = std::clamp(radius, HoleRadius + InnerRim + 2 * CornerRadius, OuterRadius - OuterRim);
    if (outerRadius_ == radius) return;
    outerRadius_ = radius;
    revealPath_ = orientation_.map(petalPath(radius));
    shadowPath_ = revealPath_.translated(0, 6);
    emit outerRadiusChanged();
    update();
}

QSizeF RadialPetal::contentSize() const {
    return contentSize_;
}

void RadialPetal::setContentSize(const QSizeF &size) {
    if (contentSize_ == size) return;
    contentSize_ = size;
    emit contentSizeChanged();
    updateContentRect();
}

QRectF RadialPetal::contentRect() const {
    return contentRect_;
}

void RadialPetal::updateContentRect() {
    if (contentSize_.isEmpty()) return;

    QPainterPathStroker padding;
    padding.setWidth(24);
    const QPainterPath usable = path_.subtracted(padding.createStroke(path_));
    const QPointF center = areaCenter(usable);
    const auto rectangle = [&](qreal scale) {
        const QSizeF size = contentSize_ * scale;
        return QRectF(center - QPointF(size.width() / 2, size.height() / 2), size);
    };

    qreal scale = 1;
    if (!usable.contains(rectangle(scale))) {
        qreal low = 0;
        qreal high = 1;
        for (int iteration = 0; iteration < 16; ++iteration) {
            const qreal candidate = (low + high) / 2;
            if (usable.contains(rectangle(candidate))) low = candidate;
            else high = candidate;
        }
        scale = low;
    }

    const QRectF bestRect = rectangle(scale);
    if (contentRect_ == bestRect) return;
    contentRect_ = bestRect;
    emit contentRectChanged();
}

bool RadialPetal::contains(const QPointF &point) const {
    if (width() <= 0 || height() <= 0) return false;
    return revealPath_.contains(QPointF(point.x() * RadialGeometry::Size / width(),
                                       point.y() * RadialGeometry::Size / height()));
}

void RadialPetal::paint(QPainter *painter) {
    painter->setRenderHint(QPainter::Antialiasing);
    painter->scale(width() / RadialGeometry::Size, height() / RadialGeometry::Size);

    const qreal opacity = fillColor_.alphaF();
    painter->setPen(QPen(QColor(0, 0, 0, qRound(12 * opacity)), 10,
                         Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
    painter->setBrush(Qt::NoBrush);
    painter->drawPath(shadowPath_);
    painter->fillPath(shadowPath_, QColor(0, 0, 0, qRound(26 * opacity)));
    painter->fillPath(revealPath_, fillColor_);
}
