#include "Actions.h"
#include "CaptureEngine.h"

CaptureAction snapshotAction() {
  return {"snapshot", "traditional", qEnvironmentVariable("SQUIGIT_SNAPSHOT_DIR"), false,
          [](CaptureEngine *controller) { controller->save(); }};
}
