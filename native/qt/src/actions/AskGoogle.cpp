#include "Actions.h"
#include "CaptureEngine.h"

CaptureAction askGoogleAction() {
  return {"ask-google", "traditional", qEnvironmentVariable("SQUIGIT_BLOB_DIR"), false,
          [](CaptureEngine *controller) { controller->save(); }};
}
