#pragma once

#include <QString>
#include <functional>

class CaptureEngine;

struct CaptureAction {
  QString id;
  QString mode;
  QString outputDirectory;
  bool autoOcr = false;
  std::function<void(CaptureEngine *)> selectionFinished;
};

CaptureAction circleToSearchAction();
CaptureAction snapshotAction();
CaptureAction scanQrAction();
CaptureAction ocrAction();
CaptureAction askGoogleAction();
CaptureAction captureAction(const QString &name);
