#pragma once

#include <atomic>
#include <functional>
#include <mutex>
#include <string>
#include <vector>

struct ma_device;

namespace squigit {

using AudioCallback = std::function<void(const std::vector<float> &pcm_data)>;

class AudioCapture {
public:
  AudioCapture();
  ~AudioCapture();

  bool init(int device_index = -1);
  bool start(AudioCallback callback);
  bool stop();
  void terminate();

  // Helper to list devices if needed later
  static std::vector<std::string> list_devices();

private:
  struct Context;
  Context *ctx = nullptr;

  std::atomic<bool> is_running{false};
  std::mutex callback_mutex;
  AudioCallback current_callback;

  // Internal callback for miniaudio
  static void data_callback(ma_device *pDevice, void *pOutput,
                            const void *pInput, unsigned int frameCount);
};

} // namespace squigit
