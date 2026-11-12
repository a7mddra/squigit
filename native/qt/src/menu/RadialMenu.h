#pragma once

#include <QObject>
#include <QQuickWindow>

class QQmlApplicationEngine;

class RadialMenu : public QObject {
  Q_OBJECT

public:
  explicit RadialMenu(QQmlApplicationEngine *engine, QObject *parent = nullptr);
  bool ready() const { return m_window != nullptr; }
  Q_INVOKABLE void show();
  Q_INVOKABLE void hide();
  Q_INVOKABLE void press(const QString &action);
  Q_INVOKABLE void playTick();

signals:
  void actionActivated(const QString &action);
  void tickRequested();

private:
  QQuickWindow *m_window = nullptr;
  QQuickWindow *m_badge = nullptr;
  bool m_positioned = false;
};
