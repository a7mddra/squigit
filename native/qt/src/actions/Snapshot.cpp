#include "Actions.h"

CaptureAction snapshotAction() {
  return {"snapshot", "traditional", qEnvironmentVariable("SQUIGIT_SNAPSHOT_DIR"), false,
          {}};
}
