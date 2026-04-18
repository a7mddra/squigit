#include <atomic>
#include <iostream>
#include <string>
#include <thread>
#include <vector>

#include "audio.hpp"
#include "inference.hpp"
#include <nlohmann/json.hpp>

#include <filesystem>
#ifdef _WIN32
#include <windows.h>
#elif __APPLE__
#include <mach-o/dyld.h>
#else
#include <unistd.h>
#endif

namespace fs = std::filesystem;
using json = nlohmann::json;

#ifndef SQUIGIT_STT_VERSION
#define SQUIGIT_STT_VERSION "0.1.0"
#endif

// Global state
std::unique_ptr<squigit::AudioCapture> audio_capture;
std::unique_ptr<squigit::InferenceEngine> inference_engine;
std::thread inference_thread;
std::atomic<bool> is_processing{false};

fs::path get_executable_dir() {
#ifdef _WIN32
    char path[MAX_PATH];
    GetModuleFileNameA(NULL, path, MAX_PATH);
    return fs::path(path).parent_path();
#elif __APPLE__
    char path[1024];
    uint32_t size = sizeof(path);
    if (_NSGetExecutablePath(path, &size) == 0) {
        return fs::path(path).parent_path();
    }
    return fs::current_path();
#else
    char path[1024];
    ssize_t count = readlink("/proc/self/exe", path, sizeof(path));
    if (count > 0 && count < sizeof(path)) {
        path[count] = '\0';
        return fs::path(path).parent_path();
    }
    return fs::current_path();
#endif
}

std::string resolve_model_path(const std::string& provided_model) {
    fs::path p(provided_model);
    if (p.is_absolute() && fs::exists(p)) return p.string();

    std::vector<fs::path> search_paths = {
        fs::current_path() / "models" / provided_model,
        fs::current_path() / provided_model,
        get_executable_dir() / "models" / provided_model,
        get_executable_dir() / "_internal" / "models" / provided_model,
        get_executable_dir() / provided_model,
        get_executable_dir().parent_path() / "share" / "squigit-stt" / "models" / provided_model,
        fs::path("/usr/share/squigit-stt/models") / provided_model,
        fs::path("/usr/local/share/squigit-stt/models") / provided_model,
        fs::path("/opt/homebrew/share/squigit-stt/models") / provided_model,
        fs::path("C:\\Program Files\\Squigit\\stt\\models") / provided_model
    };

    for (const auto& sp : search_paths) {
        if (fs::exists(sp)) return sp.string();
    }

    return provided_model;
}

void send_json(const json &j) { std::cout << j.dump() << std::endl; }

void on_transcription(const squigit::TranscriptionResult &result) {
  json j;
  j["type"] = "transcription";
  j["text"] = result.text;
  j["is_final"] = result.is_final;
  send_json(j);
}

void start_engine(const std::string &model_path, const std::string &language,
                  int device_index) {
  if (is_processing) {
    json j;
    j["type"] = "error";
    j["message"] = "Already running";
    send_json(j);
    return;
  }

  audio_capture = std::make_unique<squigit::AudioCapture>();
  inference_engine = std::make_unique<squigit::InferenceEngine>();

  squigit::InferenceParams params;
  params.model_path = model_path;
  params.language = language;

  if (!inference_engine->init(params)) {
    json j;
    j["type"] = "error";
    j["message"] = "Failed to init model";
    send_json(j);
    return;
  }

  // Emit ready status after model load
  {
    json ready;
    ready["type"] = "status";
    ready["status"] = "ready";
    ready["model"] = model_path;
    send_json(ready);
  }

  // Start Audio
  bool started = audio_capture->start([&](const std::vector<float> &pcm) {
    if (inference_engine) {
      inference_engine->add_audio(pcm);
    }
  });

  if (!started) {
    json j;
    j["type"] = "error";
    j["message"] = "Failed to start audio";
    send_json(j);
    return;
  }

  is_processing = true;

  // Start Inference Loop
  inference_thread =
      std::thread([&]() { inference_engine->run(on_transcription); });

  json j;
  j["type"] = "status";
  j["status"] = "started";
  send_json(j);
}

void stop_engine() {
  if (!is_processing)
    return;

  if (audio_capture)
    audio_capture->stop();
  if (inference_engine)
    inference_engine->stop();

  if (inference_thread.joinable()) {
    inference_thread.join();
  }

  is_processing = false;
  audio_capture.reset();
  inference_engine.reset();

  json j;
  j["type"] = "status";
  j["status"] = "stopped";
  send_json(j);
}

void print_help() {
  std::cout << "squigit-stt\n";
  std::cout << "Usage:\n";
  std::cout << "  squigit-stt --version\n";
  std::cout << "  squigit-stt --help\n";
  std::cout << "  squigit-stt  # JSON-over-stdin mode\n";
}

int main(int argc, char** argv) {
  if (argc > 1) {
    const std::string arg = argv[1];
    if (arg == "--version") {
      std::cout << SQUIGIT_STT_VERSION << std::endl;
      return 0;
    }
    if (arg == "--help" || arg == "-h") {
      print_help();
      return 0;
    }

    std::cerr << "Unknown argument: " << arg << std::endl;
    print_help();
    return 2;
  }

  std::string line;
  while (std::getline(std::cin, line)) {
    try {
      auto j = json::parse(line);
      std::string command = j["command"];

      if (command == "start") {
        std::string model = j.value("model", "ggml-tiny.en.bin");
        std::string lang = j.value("language", "en");
        int device = j.value("device_index", -1);
        start_engine(resolve_model_path(model), lang, device);
      } else if (command == "stop") {
        stop_engine();
      } else if (command == "quit") {
        stop_engine();
        break;
      }
    } catch (const std::exception &e) {
      json err;
      err["type"] = "error";
      err["message"] = e.what();
      send_json(err);
    }
  }
  return 0;
}
