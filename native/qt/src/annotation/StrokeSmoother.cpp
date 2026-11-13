#include "StrokeSmoother.h"

#include <QtMath>
#include <algorithm>
#include <array>
#include <cmath>

namespace {


using Curve = std::array<QPointF, 4>;
qreal dot(QPointF a, QPointF b) { return a.x() * b.x() + a.y() * b.y(); }
qreal len(QPointF p) { return std::hypot(p.x(), p.y()); }
QPointF unit(QPointF p) { const auto n = len(p); return n > 1e-9 ? p / n : QPointF(); }
QPointF lerp(QPointF a, QPointF b, qreal t) { return a + (b - a) * t; }

QPointF at(const Curve &c, qreal t) {
    const qreal s = 1 - t;
    return c[0] * (s*s*s) + c[1] * (3*s*s*t) + c[2] * (3*s*t*t) + c[3] * (t*t*t);
}

qreal segDist(QPointF point, QPointF start, QPointF end) {
    const auto delta = end - start;
    const qreal sq = dot(delta, delta);
    const qreal t = sq > 1e-12 ? std::clamp(dot(point - start, delta) / sq, qreal(0), qreal(1)) : 0;
    return len(point - start - delta * t);
}


QVector<QPointF> resample(const QVector<QPointF> &points, qreal spacing) {
    QVector<QPointF> result{points.first()};
    qreal remaining = spacing;
    QPointF cursor = points.first();
    for (int i = 1; i < points.size(); ++i) {
        auto delta = points[i] - cursor;
        qreal distance = len(delta);
        while (distance >= remaining && distance > 1e-9) {
            cursor += delta * (remaining / distance);
            result.append(cursor);
            delta = points[i] - cursor;
            distance = len(delta);
            remaining = spacing;
        }
        remaining -= distance;
        cursor = points[i];
    }
    if (len(points.last() - result.last()) > 1e-9) result.append(points.last());
    return result;
}


QVector<QPointF> denoise(const QVector<QPointF> &points, int first, int last, qreal sigma) {
    QVector<QPointF> result;
    result.reserve(last - first + 1);
    const int radius = std::min(last - first, std::max(1, qCeil(sigma * 3)));
    QVector<qreal> weights(radius + 1);
    qreal totalWeight = 1;
    weights[0] = 1;
    for (int j = 1; j <= radius; ++j) {
        const qreal t = j / sigma;
        weights[j] = std::exp(-t * t / 2);
        totalWeight += 2 * weights[j];
    }

    const auto extended = [&](int i) {
        if (i < first) return points[first] * 2 - points[2 * first - i];
        if (i > last) return points[last] * 2 - points[2 * last - i];
        return points[i];
    };
    for (int i = first; i <= last; ++i) {
        QPointF sum = points[i];
        for (int j = 1; j <= radius; ++j)
            sum += (extended(i - j) + extended(i + j)) * weights[j];
        result.append(sum / totalWeight);
    }
    result.first() = points[first];
    result.last() = points[last];
    return result;
}


qreal turn(const QVector<QPointF> &points, int at, int radius) {
    const auto incoming = points[at] - points[std::max(0, at - radius)];
    const auto outgoing = points[std::min(int(points.size()) - 1, at + radius)] - points[at];
    const qreal scale = len(incoming) * len(outgoing);
    return scale > 1e-9 ? std::acos(std::clamp(dot(incoming, outgoing) / scale, qreal(-1), qreal(1))) : 0;
}


QVector<int> corners(const QVector<QPointF> &points, int support) {
    QVector<qreal> scores(points.size(), 0);
    QVector<qreal> distances(points.size(), 0);
    for (int i = 1; i < points.size(); ++i)
        distances[i] = distances[i - 1] + len(points[i] - points[i - 1]);
    const auto coherent = [&](int first, int last) {
        return len(points[last] - points[first]) >= (distances[last] - distances[first]) * .85;
    };
    const int narrow = std::max(3, support / 3);
    for (int i = narrow; i < points.size() - narrow; ++i) {
        const int wide = std::min({support, i, int(points.size()) - 1 - i});
        if (wide < narrow * 2) continue;
        const qreal wideAngle = turn(points, i, wide);
        const qreal middleAngle = turn(points, i, (wide + narrow) / 2);
        const qreal narrowAngle = turn(points, i, narrow);



        if (wideAngle > 1.15 && narrowAngle > .85
            && narrowAngle > std::max(wideAngle, middleAngle) * .75
            && coherent(i - wide, i) && coherent(i, i + wide))
            scores[i] = narrowAngle + wideAngle * .25;
    }
    QVector<int> result{0};
    for (int i = 1; i < points.size() - 1; ++i) {
        if (scores[i] == 0) continue;
        bool peak = true;
        for (int j = std::max(1, i - narrow); j <= std::min(int(points.size()) - 2, i + narrow); ++j)
            if (scores[j] > scores[i] || (scores[j] == scores[i] && j < i)) { peak = false; break; }
        if (peak && i - result.last() >= narrow) result.append(i);
    }
    result.append(points.size() - 1);
    return result;
}


bool straight(const QVector<QPointF> &points, int first, int last, qreal tolerance) {
    if (last - first <= 1) return true;
    const auto chord = points[last] - points[first];
    const qreal span = len(chord);
    if (span < tolerance * 2.5) return false;
    const auto direction = chord / span;
    qreal squaredError = 0, maximumError = 0, backtrack = 0, previous = 0;
    int outside = 0;
    for (int i = first + 1; i < last; ++i) {
        const auto relative = points[i] - points[first];
        const qreal projection = dot(relative, direction);
        const qreal deviation = segDist(points[i], points[first], points[last]);
        maximumError = std::max(maximumError, deviation);
        squaredError += deviation * deviation;
        if (deviation > tolerance) ++outside;
        backtrack += std::max(qreal(0), previous - projection);
        previous = projection;
    }
    backtrack += std::max(qreal(0), previous - span);
    const int count = last - first - 1;
    return maximumError <= tolerance * 1.8
        && outside <= count / 14
        && squaredError / count <= tolerance * tolerance * .45
        && backtrack <= std::max(tolerance * 2, span * .03);
}


Curve generate(const QVector<QPointF> &p, int first, int last,
               const QVector<qreal> &u, QPointF left, QPointF right) {
    qreal c00 = 0, c01 = 0, c11 = 0, x0 = 0, x1 = 0;
    for (int i = first; i <= last; ++i) {
        const qreal t = u[i - first], s = 1 - t;
        const qreal b0 = s*s*s, b1 = 3*t*s*s, b2 = 3*t*t*s, b3 = t*t*t;
        const auto a0 = left * b1, a1 = right * b2;
        const auto residual = p[i] - p[first] * (b0 + b1) - p[last] * (b2 + b3);
        c00 += dot(a0, a0); c01 += dot(a0, a1); c11 += dot(a1, a1);
        x0 += dot(a0, residual); x1 += dot(a1, residual);
    }
    const qreal det = c00 * c11 - c01 * c01;
    qreal a = 0, b = 0;
    if (std::abs(det) > 1e-12) {
        a = (x0 * c11 - x1 * c01) / det;
        b = (c00 * x1 - c01 * x0) / det;
    }
    const qreal span = len(p[last] - p[first]);
    if (a < span * 1e-5 || b < span * 1e-5 || a > span * 3 || b > span * 3)
        a = b = span / 3;
    return {p[first], p[first] + left * a, p[last] + right * b, p[last]};
}

void fitSegment(QPainterPath &path, const QVector<QPointF> &p, int first, int last,
                QPointF left, QPointF right, qreal error, int depth) {
    if (last - first == 1) {
        const qreal handle = len(p[last] - p[first]) / 3;
        path.cubicTo(p[first] + left * handle, p[last] + right * handle, p[last]);
        return;
    }
    if (depth >= 32) {
        for (int i = first; i < last; ++i) {
            const auto next = i + 1 == last ? right : unit(p[i] - p[i + 2]);
            const qreal handle = len(p[i + 1] - p[i]) / 3;
            path.cubicTo(p[i] + left * handle, p[i + 1] + next * handle, p[i + 1]);
            left = -next;
        }
        return;
    }
    QVector<qreal> u(last - first + 1, 0);
    for (int i = 1; i < u.size(); ++i) u[i] = u[i - 1] + len(p[first + i] - p[first + i - 1]);
    const qreal total = u.last();
    if (total < 1e-9) return;
    for (auto &t : u) t /= total;

    Curve curve;
    int split = (first + last) / 2;
    for (int iteration = 0; iteration < 6; ++iteration) {
        curve = generate(p, first, last, u, left, right);
        qreal maxError = 0;
        for (int i = first + 1; i < last; ++i) {
            const auto v = at(curve, u[i - first]) - p[i];
            const qreal distance = dot(v, v);
            if (distance > maxError) { maxError = distance; split = i; }
        }
        if (maxError <= error * error) {
            path.cubicTo(curve[1], curve[2], curve[3]);
            return;
        }
        if (maxError > error * error * 16) break;
        auto next = u;
        bool ordered = true;
        for (int i = 1; i < u.size() - 1; ++i) {
            const qreal t = u[i], s = 1 - t;
            const auto d1 = (curve[1] - curve[0]) * (3*s*s)
                          + (curve[2] - curve[1]) * (6*s*t) + (curve[3] - curve[2]) * (3*t*t);
            const auto d2 = (curve[2] - curve[1] * 2 + curve[0]) * (6*s)
                          + (curve[3] - curve[2] * 2 + curve[1]) * (6*t);
            const auto delta = at(curve, t) - p[first + i];
            const qreal denominator = dot(d1, d1) + dot(delta, d2);
            if (std::abs(denominator) > 1e-10)
                next[i] = std::clamp(t - dot(delta, d1) / denominator, qreal(0), qreal(1));
            if (next[i] <= next[i - 1]) ordered = false;
        }
        if (!ordered || next[next.size() - 2] >= 1) break;
        u = next;
    }
    split = std::clamp(split, first + 1, last - 1);
    QPointF center = unit(p[split - 1] - p[split + 1]);
    if (len(center) < 1e-9) center = unit(p[split - 1] - p[split]);
    fitSegment(path, p, first, split, left, center, error, depth + 1);
    fitSegment(path, p, split, last, -center, right, error, depth + 1);
}

}






QVector<QPointF> StrokeSmoother::streamline(const QVector<QPointF> &points, qreal factor) {
    if (points.size() < 2 || factor <= 0) return points;













    const qreal alpha = std::clamp(1.0 - factor, qreal(0.08), qreal(1.0));
    const int n = points.size();


    QVector<QPointF> forward(n);
    forward[0] = points[0];
    for (int i = 1; i < n; ++i)
        forward[i] = lerp(forward[i - 1], points[i], alpha);


    QVector<QPointF> backward(n);
    backward[n - 1] = points[n - 1];
    for (int i = n - 2; i >= 0; --i)
        backward[i] = lerp(backward[i + 1], points[i], alpha);




    QVector<QPointF> result(n);
    result[0] = points[0];
    result[n - 1] = points[n - 1];
    for (int i = 1; i < n - 1; ++i)
        result[i] = (forward[i] + backward[i]) * 0.5;

    return result;
}

QPainterPath StrokeSmoother::fit(const QVector<QPointF> &points, qreal smoothing, qreal pixelSize) {
    QPainterPath path;
    if (points.isEmpty()) return path;
    path.moveTo(points.first());
    if (points.size() == 1) {
        path.lineTo(points.first() + QPointF(.001, 0));
        return path;
    }

    const qreal strength = std::clamp(smoothing, qreal(0), qreal(1));
    const qreal pixel = std::max(qreal(.01), pixelSize);
    const qreal spacing = pixel * 1.25;

    const auto sampled = resample(points, spacing);
    if (sampled.size() < 3) {
        path.lineTo(points.last());
        return path;
    }


    const qreal sigma = (.8 + strength * 6) / 1.25;


    const auto detection = denoise(sampled, 0, sampled.size() - 1, 1.4);
    const auto turns = corners(detection, qRound((14 + strength * 8) / 1.25));

    for (int section = 1; section < turns.size(); ++section) {
        const int first = turns[section - 1], last = turns[section];
        auto cleaned = denoise(sampled, first, last, std::max(qreal(.5), sigma));
        const qreal span = len(cleaned.last() - cleaned.first());


        const qreal lineTolerance = pixel * (.8 + strength * 4) + std::min(span * .015, pixel * 10) * strength;
        if (straight(cleaned, 0, cleaned.size() - 1, lineTolerance)) {
            path.lineTo(cleaned.last());
            continue;
        }

        const int tangentWindow = std::min(int(cleaned.size()) - 1, 2);
        const auto left = unit(cleaned[tangentWindow] - cleaned.first());
        const auto right = unit(cleaned[cleaned.size() - 1 - tangentWindow] - cleaned.last());
        fitSegment(path, cleaned, 0, cleaned.size() - 1, left, right,
                   pixel * (.5 + strength * 2.5), 0);
    }
    return path;
}
