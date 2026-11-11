#include "DisplayGuard.h"
#include <QGuiApplication>
#include <QScreen>

DisplayGuard::DisplayGuard(QObject *parent) : QObject(parent) {
  for (auto *screen : qGuiApp->screens())
    watch(screen);
  connect(qGuiApp, &QGuiApplication::screenAdded, this, [this](QScreen *screen) {
    watch(screen);
    emit changed();
  });
  connect(qGuiApp, &QGuiApplication::screenRemoved, this, [this](QScreen *) {
    emit changed();
  });
}

void DisplayGuard::watch(QScreen *screen) {
  connect(screen, &QScreen::geometryChanged, this, [this](const QRect &) { emit changed(); });
  connect(screen, &QScreen::virtualGeometryChanged, this, [this](const QRect &) { emit changed(); });
  connect(screen, &QScreen::logicalDotsPerInchChanged, this, [this](qreal) { emit changed(); });
  connect(screen, &QScreen::physicalDotsPerInchChanged, this, [this](qreal) { emit changed(); });
}
