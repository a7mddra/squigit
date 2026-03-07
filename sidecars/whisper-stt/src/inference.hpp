#pragma once

#include <functional>
#include <memory>
#include <string>
#include <vector>

struct whisper_context;

namespace squigit {

struct InferenceParams {
  std::string model_path;
  std::string language = "en";
  bool translate = false;
  int n_threads = 4;
};

struct TranscriptionResult {
  std::string text;
  bool is_final; // true if segment is complete
  int64_t t0;
  int64_t t1;
};

using TranscriptionCallback = std::function<void(const TranscriptionResult &)>;

class InferenceEngine {
public:
  InferenceEngine();
  ~InferenceEngine();

  bool init(const InferenceParams &params);

  // Add audio to the buffer. This is thread-safe and non-blocking.
  void add_audio(const std::vector<float> &pcm_data);

  // Run the inference loop. This blocks until stop() is called.
  void run(TranscriptionCallback callback);

  void stop();

private:
  struct Impl;
  Impl *impl;
};

} // namespace squigit
