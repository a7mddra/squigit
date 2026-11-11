#include "Actions.h"
#include "CaptureEngine.h"

CaptureAction circleToSearchAction() {
  return {"circle-to-search", "squiggle", qEnvironmentVariable("SQUIGIT_BLOB_DIR"), false,
          [](CaptureEngine *controller) { controller->save(); }};
}
