#pragma once

#include <QJsonDocument>
#include <QJsonObject>
#include <iostream>

inline void sendEvent(const QJsonObject &event) {
  std::cout << QJsonDocument(event).toJson(QJsonDocument::Compact).toStdString()
            << std::endl;
}
