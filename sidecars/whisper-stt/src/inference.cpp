// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

#include "inference.hpp"
#include "whisper.h"
#include <algorithm>
#include <atomic>
#include <chrono>
#include <cmath>
#include <condition_variable>
#include <iostream>
#include <mutex>
#include <thread>
#include <vector>

namespace squigit {

struct InferenceEngine::Impl {
  struct whisper_context *ctx = nullptr;
  InferenceParams params;

  std::vector<float> audio_buffer;
  std::mutex audio_mutex;

  std::atomic<bool> running{false};
  std::atomic<bool> should_stop{false};

  // Streaming bookkeeping
  size_t processed_samples = 0; // samples we've already "consumed"
  std::condition_variable cv;
  std::mutex cv_mutex;

  // VAD / finality bookkeeping
  bool had_speech = false;
  std::chrono::steady_clock::time_point last_voice_time =
      std::chrono::steady_clock::now();

  // Configuration
  int sample_rate = 16000;
  const int SILENCE_THRESHOLD_MS = 700;
  const int MIN_SPEECH_MS =
      150; // must have at least 150ms speech to consider final
  const size_t MAX_BUFFER_SECONDS = 60; // safety cap
};

InferenceEngine::InferenceEngine() : impl(new Impl()) {}

InferenceEngine::~InferenceEngine() {
  stop();
  if (impl->ctx) {
    whisper_free(impl->ctx);
  }
  delete impl;
}

bool InferenceEngine::init(const InferenceParams &params) {
  impl->params = params;
  impl->sample_rate = 16000; // model expected sample rate

  struct whisper_context_params cparams = whisper_context_default_params();
  impl->ctx =
      whisper_init_from_file_with_params(params.model_path.c_str(), cparams);

  if (!impl->ctx) {
    std::cerr << "Failed to initialize whisper context from "
              << params.model_path << std::endl;
    return false;
  }

  return true;
}

// Compute RMS over given buffer
static float calculate_rms_samples(const float *data, size_t n) {
  if (n == 0)
    return 0.0f;
  double sum = 0.0;
  for (size_t i = 0; i < n; ++i) {
    double v = data[i];
    sum += v * v;
  }
  return static_cast<float>(std::sqrt(sum / n));
}

void InferenceEngine::add_audio(const std::vector<float> &pcm_data) {
  {
    std::lock_guard<std::mutex> lock(impl->audio_mutex);
    // Append new audio
    impl->audio_buffer.insert(impl->audio_buffer.end(), pcm_data.begin(),
                              pcm_data.end());

    // Enforce max buffer cap (drop oldest if needed)
    const size_t max_samples = impl->MAX_BUFFER_SECONDS * impl->sample_rate;
    if (impl->audio_buffer.size() > max_samples) {
      size_t remove = impl->audio_buffer.size() - max_samples;
      if (remove >= impl->audio_buffer.size()) {
        impl->audio_buffer.clear();
        impl->processed_samples = 0;
      } else {
        impl->audio_buffer.erase(impl->audio_buffer.begin(),
                                 impl->audio_buffer.begin() + remove);
        // Scale processed_samples accordingly
        if (impl->processed_samples > remove)
          impl->processed_samples -= remove;
        else
          impl->processed_samples = 0;
      }
    }
  }
  // Notify inference thread
  impl->cv.notify_one();
}

void InferenceEngine::stop() {
  impl->should_stop = true;
  impl->cv.notify_one();
}

// Worker run loop implementing:
// - Event-driven wake (condition variable)
// - Process only new samples with 1s overlap
// - RMS-based VAD + silence timeout to declare final segments
void InferenceEngine::run(TranscriptionCallback callback) {
  impl->running = true;
  impl->should_stop = false;

  const size_t sample_rate = impl->sample_rate;
  const size_t one_second_samples = sample_rate;
  const size_t min_chunk_samples =
      sample_rate / 2; // 0.5s minimum to attempt inference

  std::vector<float> work_buf;

  while (!impl->should_stop) {
    // Wait for audio or timeout
    {
      std::unique_lock<std::mutex> lk(impl->cv_mutex);
      impl->cv.wait_for(lk, std::chrono::milliseconds(400), [&]() {
        std::lock_guard<std::mutex> al(impl->audio_mutex);
        return impl->should_stop ||
               (impl->audio_buffer.size() >
                impl->processed_samples + (min_chunk_samples / 2));
      });
    }

    if (impl->should_stop)
      break;

    // Move only new samples, but keep 1s overlap for context
    {
      std::lock_guard<std::mutex> lock(impl->audio_mutex);
      size_t total = impl->audio_buffer.size();
      if (total <= impl->processed_samples) {
        continue;
      }

      size_t start = 0;
      if (impl->processed_samples > one_second_samples) {
        start =
            impl->processed_samples - one_second_samples; // 1 second overlap
      } else {
        start = 0;
      }

      // Copy into work_buf
      work_buf.assign(impl->audio_buffer.begin() + start,
                      impl->audio_buffer.end());
      // Mark consumed up to current total
      impl->processed_samples = total;
    }

    if (work_buf.size() < min_chunk_samples) {
      continue;
    }

    // VAD check — compute RMS over last 300ms of work_buf
    size_t window_samples =
        std::min<size_t>(work_buf.size(), (size_t)(impl->sample_rate * 0.3));
    float rms = calculate_rms_samples(
        work_buf.data() + (work_buf.size() - window_samples), window_samples);

    // Tune threshold for your environment (0.003 - 0.02 typical)
    const float ENERGY_THRESHOLD = 0.003f;

    bool voice_now = (rms >= ENERGY_THRESHOLD);

    if (voice_now) {
      impl->had_speech = true;
      impl->last_voice_time = std::chrono::steady_clock::now();
    }

    // Run whisper inference on work_buf
    whisper_full_params wparams =
        whisper_full_default_params(WHISPER_SAMPLING_GREEDY);
    wparams.print_progress = false;
    wparams.print_special = false;
    wparams.print_realtime = false;
    wparams.print_timestamps = false;
    wparams.translate = impl->params.translate;
    wparams.language = impl->params.language.c_str();
    wparams.n_threads = impl->params.n_threads;
    wparams.no_context = true;
    wparams.single_segment = false;

    if (whisper_full(impl->ctx, wparams, work_buf.data(),
                     (int)work_buf.size()) != 0) {
      std::cerr << "whisper_full() failed during inference." << std::endl;
      TranscriptionResult err;
      err.text = "";
      err.is_final = false;
      err.t0 = err.t1 = 0;
      callback(err);
      continue;
    }

    // Gather the output text
    const int n_segments = whisper_full_n_segments(impl->ctx);
    std::string text;
    for (int i = 0; i < n_segments; ++i) {
      const char *seg_txt = whisper_full_get_segment_text(impl->ctx, i);
      if (seg_txt)
        text += seg_txt;
    }

    // Decide finality: if we previously had speech and silence has lasted >
    // threshold => final
    bool is_final = false;
    if (impl->had_speech) {
      auto now = std::chrono::steady_clock::now();
      auto silence_ms = std::chrono::duration_cast<std::chrono::milliseconds>(
                            now - impl->last_voice_time)
                            .count();
      if (silence_ms >= impl->SILENCE_THRESHOLD_MS) {
        is_final = true;
      }
    }

    // Callback with result
    TranscriptionResult result;
    result.text = text;
    result.is_final = is_final;
    result.t0 = 0;
    result.t1 = 0;
    callback(result);

    if (is_final) {
      // Clear buffer & reset state to avoid reprocessing
      {
        std::lock_guard<std::mutex> lock(impl->audio_mutex);
        impl->audio_buffer.clear();
        impl->processed_samples = 0;
      }
      impl->had_speech = false;
    }
  }

  impl->running = false;
}

} // namespace squigit
