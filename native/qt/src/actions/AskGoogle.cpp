#include "Actions.h"

CaptureAction askGoogleAction() {
  return {"ask-google", "traditional", qEnvironmentVariable("SQUIGIT_BLOB_DIR"), false,
          {}};
}
