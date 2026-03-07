#include "audio.hpp"
#include <iostream>

#define MINIAUDIO_IMPLEMENTATION
#include "miniaudio.h"

namespace squigit {

struct AudioCapture::Context {
  ma_context context;
  ma_device device;
  ma_device_config deviceConfig;
  bool device_inited = false;
  bool context_inited = false;
  AudioCapture *parent = nullptr;
};

AudioCapture::AudioCapture() : ctx(new Context()) { ctx->parent = this; }

AudioCapture::~AudioCapture() {
  terminate();
  delete ctx;
}

void AudioCapture::data_callback(ma_device *pDevice, void *pOutput,
                                 const void *pInput, unsigned int frameCount) {
  ma_device *device = pDevice;
  AudioCapture *self = (AudioCapture *)device->pUserData;

  if (!self || !self->is_running)
    return;

  // Assuming float32 input from config
  const float *input_floats = (const float *)pInput;

  // Copy data to vector
  std::vector<float> data(input_floats, input_floats + frameCount);

  std::lock_guard<std::mutex> lock(self->callback_mutex);
  if (self->current_callback) {
    self->current_callback(data);
  }
}

bool AudioCapture::init(int device_index) {
  if (ma_context_init(NULL, 0, NULL, &ctx->context) != MA_SUCCESS) {
    std::cerr << "Failed to initialize miniaudio context." << std::endl;
    return false;
  }
  ctx->context_inited = true;

  ctx->deviceConfig = ma_device_config_init(ma_device_type_capture);
  ctx->deviceConfig.capture.format = ma_format_f32;
  ctx->deviceConfig.capture.channels = 1;
  ctx->deviceConfig.sampleRate = 16000; // Whisper standard
  ctx->deviceConfig.dataCallback = data_callback;
  ctx->deviceConfig.pUserData = this;

  // Handle device selection
  if (device_index >= 0) {
    ma_device_info *pCaptureInfos = nullptr;
    ma_uint32 captureCount = 0;
    if (ma_context_get_devices(&ctx->context, nullptr, nullptr, &pCaptureInfos,
                               &captureCount) == MA_SUCCESS) {
      if (device_index < (int)captureCount) {
        ctx->deviceConfig.capture.pDeviceID = &pCaptureInfos[device_index].id;
        std::cerr << "[audio] Using device " << device_index << ": "
                  << pCaptureInfos[device_index].name << std::endl;
      } else {
        std::cerr << "[audio] Invalid device index " << device_index
                  << ", using default" << std::endl;
      }
    }
  }

  if (ma_device_init(&ctx->context, &ctx->deviceConfig, &ctx->device) !=
      MA_SUCCESS) {
    std::cerr << "Failed to initialize capture device." << std::endl;
    return false;
  }
  ctx->device_inited = true;

  return true;
}

bool AudioCapture::start(AudioCallback callback) {
  if (!ctx->device_inited) {
    if (!init())
      return false;
  }

  {
    std::lock_guard<std::mutex> lock(callback_mutex);
    current_callback = callback;
  }

  if (ma_device_start(&ctx->device) != MA_SUCCESS) {
    std::cerr << "Failed to start device." << std::endl;
    return false;
  }

  is_running = true;
  return true;
}

bool AudioCapture::stop() {
  if (!is_running)
    return true;

  if (ma_device_stop(&ctx->device) != MA_SUCCESS) {
    return false;
  }

  is_running = false;
  return true;
}

void AudioCapture::terminate() {
  stop();
  if (ctx->device_inited) {
    ma_device_uninit(&ctx->device);
    ctx->device_inited = false;
  }
  if (ctx->context_inited) {
    ma_context_uninit(&ctx->context);
    ctx->context_inited = false;
  }
}

std::vector<std::string> AudioCapture::list_devices() {
  // Implementation for listing devices if needed
  return {};
}

} // namespace squigit
