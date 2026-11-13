#pragma once

#include "actions/Actions.h"
#include <QHash>
#include <QPointer>
#include <QQmlApplicationEngine>
#include <QQmlComponent>
#include <QQuickWindow>
#include <memory>

class BackgroundImageProvider;
class CaptureEngine;
class Grabber;
class RadialMenu;

class CaptureRuntime : public QObject {
  Q_OBJECT

public:
  explicit CaptureRuntime(QObject *parent = nullptr);
  ~CaptureRuntime() override;
  bool ready() const;
  void acceptMessage(const QByteArray &line);
  void shutdown();

private:
  struct RetiredCapture {
    QList<QQuickWindow *> windows;
    QList<CaptureEngine *> controllers;
  };
  void showMenu();
  void startAction(const QString &name);
  void beginCapture(const CaptureAction &action, quint64 generation);
  void finishCapture(const QString &status, const QString &detail = {},
                     const QString &prompt = {});
  void releaseCapture(const QString &session);
  QQmlApplicationEngine m_qml;
  QQmlComponent m_captureComponent;
  std::unique_ptr<RadialMenu> m_menu;
  Grabber *m_grabber = nullptr;
  BackgroundImageProvider *m_backgrounds = nullptr;
  QList<CaptureEngine *> m_controllers;
  QList<QQuickWindow *> m_windows;
  QHash<QString, RetiredCapture> m_retired;
  QPointer<QQuickWindow> m_chosenWindow;
  CaptureAction m_action;
  QString m_session;
  quint64 m_generation = 0;
  bool m_selectionFinished = false;
};
