#include "Actions.h"

CaptureAction captureAction(const QString &name) {
  if (name == "Circle to Search" || name == "circle-to-search")
    return circleToSearchAction();
  if (name == "Snapshot" || name == "snapshot")
    return snapshotAction();
  if (name == "Scan QR" || name == "scan-qr")
    return scanQrAction();
  if (name == "OCR" || name == "ocr")
    return ocrAction();
  if (name == "Ask Google" || name == "ask-google")
    return askGoogleAction();
  return {};
}
