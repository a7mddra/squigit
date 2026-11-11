#include "Actions.h"

CaptureAction captureAction(const QString &name) {
  if (name == "Circle to Search")
    return circleToSearchAction();
  if (name == "Snapshot")
    return snapshotAction();
  if (name == "Scan QR")
    return scanQrAction();
  if (name == "OCR")
    return ocrAction();
  if (name == "Ask Google")
    return askGoogleAction();
  return {};
}
