#pragma once

#include <QColor>
#include <QPainterPath>
#include <QQuickPaintedItem>
#include <QSizeF>
#include <QTransform>

class RadialPetal : public QQuickPaintedItem {
    Q_OBJECT
    Q_PROPERTY(QColor fillColor READ fillColor WRITE setFillColor NOTIFY fillColorChanged)
    Q_PROPERTY(qreal angle READ angle WRITE setAngle NOTIFY angleChanged)
    Q_PROPERTY(qreal outerRadius READ outerRadius WRITE setOuterRadius NOTIFY outerRadiusChanged)
    Q_PROPERTY(QSizeF contentSize READ contentSize WRITE setContentSize NOTIFY contentSizeChanged)
    Q_PROPERTY(QRectF contentRect READ contentRect NOTIFY contentRectChanged)

public:
    explicit RadialPetal(QQuickItem *parent = nullptr);

    QColor fillColor() const;
    void setFillColor(const QColor &color);
    qreal angle() const;
    void setAngle(qreal angle);
    qreal outerRadius() const;
    void setOuterRadius(qreal radius);
    QSizeF contentSize() const;
    void setContentSize(const QSizeF &size);
    QRectF contentRect() const;
    bool contains(const QPointF &point) const override;
    void paint(QPainter *painter) override;

signals:
    void fillColorChanged();
    void angleChanged();
    void outerRadiusChanged();
    void contentSizeChanged();
    void contentRectChanged();

private:
    void updateContentRect();

    QPainterPath basePath_;
    QPainterPath path_;
    QPainterPath revealPath_;
    QPainterPath shadowPath_;
    QTransform orientation_;
    QColor fillColor_;
    qreal angle_ = 0;
    qreal outerRadius_;
    QSizeF contentSize_;
    QRectF contentRect_;
};
