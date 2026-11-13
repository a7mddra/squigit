#include "AnnotationGeometry.h"
#include "AnnotationDocument.h"

#include <QAbstractTextDocumentLayout>
#include <QGlyphRun>
#include <QGuiApplication>
#include <QPainterPathStroker>
#include <QTextBlock>
#include <QTextDocument>
#include <QTextLayout>
#include <QTextOption>
#include <QtMath>
#include <algorithm>
#include <cmath>
#include <optional>

namespace {
struct Contour {
  QPainterPath path;
  QPointF interior;
  long double area = 0;
  int parent = -1;
  int depth = 0;
};

bool boundsOverlap(const QRectF &a, const QRectF &b) {
  return a.left() <= b.right() && b.left() <= a.right()
      && a.top() <= b.bottom() && b.top() <= a.bottom();
}

long double cross(QPointF a, QPointF b, QPointF c) {
  return (static_cast<long double>(b.x()) - a.x()) * (static_cast<long double>(c.y()) - a.y())
      - (static_cast<long double>(b.y()) - a.y()) * (static_cast<long double>(c.x()) - a.x());
}

bool segmentsTouch(QPointF a, QPointF b, QPointF c, QPointF d) {
  if (!boundsOverlap(QRectF(a, b).normalized(), QRectF(c, d).normalized())) return false;
  const auto abC = cross(a, b, c), abD = cross(a, b, d);
  const auto cdA = cross(c, d, a), cdB = cross(c, d, b);
  return ((abC <= 0 && abD >= 0) || (abC >= 0 && abD <= 0))
      && ((cdA <= 0 && cdB >= 0) || (cdA >= 0 && cdB <= 0));
}

bool connected(const QPainterPath &a, const QPainterPath &b) {
  if (!boundsOverlap(a.boundingRect(), b.boundingRect())) return false;
  if (a.intersects(b)) return true;
  const auto aPolygons = a.toSubpathPolygons(), bPolygons = b.toSubpathPolygons();
  for (const auto &left : aPolygons) {
    for (const auto &right : bPolygons) {
      if (!boundsOverlap(left.boundingRect(), right.boundingRect())) continue;
      for (qsizetype i = 0; i < left.size(); ++i)
        for (qsizetype j = 0; j < right.size(); ++j)
          if (segmentsTouch(left[i], left[(i + 1) % left.size()], right[j], right[(j + 1) % right.size()])) return true;
    }
  }
  return false;
}

std::optional<QPointF> interiorPoint(const QPolygonF &polygon, const QPainterPath &path) {
  QVector<qreal> rows;
  for (const auto &point : polygon) rows.append(point.y());
  std::sort(rows.begin(), rows.end());
  rows.erase(std::unique(rows.begin(), rows.end()), rows.end());
  for (qsizetype row = 1; row < rows.size(); ++row) {
    const qreal y = rows[row - 1] + (rows[row] - rows[row - 1]) / 2;
    if (y <= rows[row - 1] || y >= rows[row]) continue;
    QVector<qreal> crossings;
    for (qsizetype index = 0; index < polygon.size(); ++index) {
      const auto a = polygon[index], b = polygon[(index + 1) % polygon.size()];
      if ((a.y() < y && b.y() > y) || (b.y() < y && a.y() > y))
        crossings.append(a.x() + (y - a.y()) * (b.x() - a.x()) / (b.y() - a.y()));
    }
    std::sort(crossings.begin(), crossings.end());
    for (qsizetype index = 1; index < crossings.size(); index += 2) {
      const QPointF point(crossings[index - 1] + (crossings[index] - crossings[index - 1]) / 2, y);
      if (path.contains(point)) return point;
    }
  }
  return std::nullopt;
}
}

namespace AnnotationGeometry {
QPainterPath textPath(const Annotation &item) {
  QTextDocument document;
  AnnotationGeometry::configureText(item, document);
  document.documentLayout()->documentSize();
  QPainterPath result;
  const auto inner = item.rect.adjusted(6, 6, -6, -6);
  for (auto block = document.begin(); block.isValid(); block = block.next()) {
    const auto *layout = block.layout();
    if (!layout) continue;
    const auto origin = inner.topLeft() + document.documentLayout()->blockBoundingRect(block).topLeft();
    for (const auto &run : layout->glyphRuns()) {
      const auto font = run.rawFont();
      const auto glyphs = run.glyphIndexes();
      const auto positions = run.positions();
      for (qsizetype index = 0; index < std::min(glyphs.size(), positions.size()); ++index) {
        const auto position = origin + positions[index];
        result = result.united(QTransform::fromTranslate(position.x(), position.y()).map(font.pathForGlyph(glyphs[index])));
      }
    }
  }
  QPainterPath clip;
  clip.addRect(inner);
  return result.intersected(clip);
}

QPainterPath outline(const Annotation &item, qreal extra) {
  QPainterPathStroker stroker;
  stroker.setWidth(item.width + extra);
  stroker.setCapStyle(item.tool == "highlighter" ? Qt::FlatCap : Qt::RoundCap);
  stroker.setJoinStyle(Qt::RoundJoin);
  return stroker.createStroke(item.path);
}

void configureText(const Annotation &item, QTextDocument &document) {
  QFont font = QGuiApplication::font();
  font.setPixelSize(std::max(1, qRound(item.width)));
  document.setDefaultFont(font);
  document.setDocumentMargin(0);
  QTextOption option;
  option.setWrapMode(QTextOption::WrapAtWordBoundaryOrAnywhere);
  document.setDefaultTextOption(option);
  document.setPlainText(item.text);
  document.setTextWidth(item.rect.width() - 12);
}

QPainterPath ink(const Annotation &item) {
  if (item.opacity <= 0) return {};
  if (item.tool == "text") return item.path;
  if (item.tool == "counter") {
    QPainterPath ellipse;
    ellipse.addEllipse(item.rect);
    QPainterPathStroker border;
    border.setWidth(2);
    return ellipse.united(border.createStroke(ellipse));
  }
  if (item.tool.startsWith("fill") || item.tool == "highlightRectangle") return item.path;
  if (item.tool == "blur") {
    QPainterPath clip;
    clip.addRect(item.blurRect);
    return outline(item).intersected(clip);
  }
  return outline(item);
}

QPainterPath visibleInk(const Annotation &item) {
  auto path = ink(item);
  if (!item.fragment.isEmpty()) path = path.intersected(item.fragment);
  if (!item.erased.isEmpty()) path = path.subtracted(item.erased);
  return path;
}

QVector<QPainterPath> connectedComponents(const QPainterPath &path) {
  if (path.isEmpty()) return {};
  QVector<Contour> contours;
  for (const auto &polygon : path.simplified().toSubpathPolygons()) {
    long double area = 0;
    for (qsizetype index = 0; index < polygon.size(); ++index) {
      const auto a = polygon[index], b = polygon[(index + 1) % polygon.size()];
      area += static_cast<long double>(a.x()) * b.y() - static_cast<long double>(b.x()) * a.y();
    }
    if (area == 0) continue;
    Contour contour;
    contour.path.addPolygon(polygon);
    contour.path.closeSubpath();
    contour.path.setFillRule(Qt::OddEvenFill);
    contour.area = std::abs(area);
    const auto interior = interiorPoint(polygon, contour.path);
    if (!interior) return {path};
    contour.interior = *interior;
    contours.append(contour);
  }
  for (int index = 0; index < contours.size(); ++index) {
    auto &contour = contours[index];
    for (int other = 0; other < contours.size(); ++other) {
      if (contours[other].area <= contour.area || !contours[other].path.contains(contour.interior)) continue;
      if (contour.parent < 0 || contours[other].area < contours[contour.parent].area) contour.parent = other;
    }
  }
  for (auto &contour : contours) {
    for (int parent = contour.parent; parent >= 0; parent = contours[parent].parent) ++contour.depth;
  }
  QVector<QPainterPath> components;
  for (int index = 0; index < contours.size(); ++index) {
    if (contours[index].depth % 2 != 0) continue;
    auto component = contours[index].path;
    for (const auto &hole : contours)
      if (hole.parent == index) component.addPath(hole.path);
    components.append(component);
  }
  QVector<int> groups(components.size());
  for (int index = 0; index < groups.size(); ++index) groups[index] = index;
  const auto root = [&groups](int index) {
    while (groups[index] != index) index = groups[index];
    return index;
  };
  for (int index = 0; index < components.size(); ++index)
    for (int other = index + 1; other < components.size(); ++other)
      if (root(index) != root(other) && connected(components[index], components[other])) groups[root(other)] = root(index);
  QVector<QPainterPath> result;
  QHash<int, int> destinations;
  for (int index = 0; index < components.size(); ++index) {
    const int group = root(index);
    if (!destinations.contains(group)) {
      destinations.insert(group, result.size());
      result.append(components[index]);
    } else {
      auto &component = result[destinations.value(group)];
      component = component.united(components[index]);
    }
  }
  return result.isEmpty() ? QVector<QPainterPath>{path} : result;
}
}
