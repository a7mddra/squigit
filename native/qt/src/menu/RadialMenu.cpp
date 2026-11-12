#include "RadialMenu.h"
#include "RadialGeometry.h"
#include "RadialPetal.h"
#include "WindowEffects.h"

#include <QCursor>
#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QQmlComponent>
#include <QQmlContext>
#include <QRegion>
#include <QScreen>
#include <QVariantMap>
#include <algorithm>
#include <iostream>

RadialMenu::RadialMenu(QQmlApplicationEngine *engine, QObject *parent)
    : QObject(parent) {
  qmlRegisterType<RadialPetal>("RadialMenu.Native", 1, 0, "RadialPetal");
  auto *screen = QGuiApplication::screenAt(QCursor::pos());
  if (!screen)
    screen = QGuiApplication::primaryScreen();
  int diameter = 400;
  if (screen) {
    const auto available = screen->availableGeometry();
    diameter = std::min(diameter, std::max(1, std::min(available.width(), available.height()) - 64));
  }
  const QVariantMap config{
      {"diameter", diameter}, {"circle-opacity", 0.45},
      {"button-opacity", 0.75}, {"hover-opacity", 0.85},
      {"inner-radius", RadialGeometry::HoleRadius},
      {"petal-outer-radius", RadialGeometry::OuterRadius - RadialGeometry::OuterRim},
      {"outer-rim", RadialGeometry::OuterRim}};
  auto *context = new QQmlContext(engine->rootContext(), this);
  context->setContextProperty("menuConfig", config);
  context->setContextProperty("menuActions", this);
  QQmlComponent component(engine, QUrl("qrc:/menu/RadialWindow.qml"));
  m_window = qobject_cast<QQuickWindow *>(component.create(context));
  if (!m_window) {
    std::cerr << component.errorString().toStdString() << std::endl;
    return;
  }
  m_window->QObject::setParent(this);
  m_window->create();
  disableWindowAnimations(m_window);
  m_badge = m_window->findChild<QQuickWindow *>(QStringLiteral("hoverBadge"));
  if (m_badge) {
    m_badge->create();
    disableWindowAnimations(m_badge);
  }
  const QPointF center(m_window->width() / 2.0, m_window->height() / 2.0);
  const qreal radius = diameter * RadialGeometry::HoleRadius / RadialGeometry::Size - 1;
  const QRect hole = QRectF(center.x() - radius, center.y() - radius, radius * 2, radius * 2).toRect();
  const QRegion outer(QRect(11, 11, diameter + 2, diameter + 2), QRegion::Ellipse);
  m_window->setMask(outer.subtracted(QRegion(hole, QRegion::Ellipse)));
}

void RadialMenu::show() {
  if (!m_window)
    return;
  auto *screen = QGuiApplication::screenAt(QCursor::pos());
  if (!screen)
    screen = QGuiApplication::primaryScreen();
  if (screen && (!m_positioned || m_window->screen() != screen ||
                 !screen->availableGeometry().intersects(m_window->geometry()))) {
    m_window->setScreen(screen);
    m_window->setPosition(screen->availableGeometry().center() -
                         QPoint(m_window->width() / 2, m_window->height() / 2));
    m_positioned = true;
  }
  m_window->show();
  m_window->raise();
  m_window->requestActivate();
}

void RadialMenu::hide() {
  if (!m_window)
    return;
  if (m_badge) {
    m_badge->setOpacity(0);
    m_badge->hide();
  }
  m_window->setOpacity(0);
  m_window->hide();
}

void RadialMenu::press(const QString &action) {
  emit actionActivated(action);
}

void RadialMenu::playTick() {
  if (m_window && m_window->isVisible())
    emit tickRequested();
}
