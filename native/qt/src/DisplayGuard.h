#pragma once

#include <QObject>

class QScreen;

class DisplayGuard : public QObject {
  Q_OBJECT

public:
  explicit DisplayGuard(QObject *parent = nullptr);

signals:
  void changed();

private:
  void watch(QScreen *screen);
};
