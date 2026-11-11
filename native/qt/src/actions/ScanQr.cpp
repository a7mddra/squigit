#include "Actions.h"
#include "CaptureEngine.h"

CaptureAction scanQrAction() {
  return {"scan-qr", "traditional", {}, false,
          [](CaptureEngine *controller) { controller->scanQr(); }};
}
