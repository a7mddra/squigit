cmake_minimum_required(VERSION 3.14)
project(squigit-stt VERSION 0.1.0)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Optimization flags
if(MSVC)
    add_compile_options(/O2)
else()
    add_compile_options(-O3)
endif()

include(FetchContent)

# Build shared libs so runtime dependencies can be packaged under _internal/.
set(BUILD_SHARED_LIBS ON CACHE BOOL "Build shared libraries" FORCE)
set(
    GGML_NATIVE
    OFF
    CACHE BOOL "Disable host-specific instructions for portable CPU binaries"
    FORCE
)
set(GGML_OPENMP ON CACHE BOOL "Enable OpenMP for CPU performance" FORCE)

# 1. whisper.cpp
# Pinned to a specific version for stability as requested
FetchContent_Declare(
  whisper_cpp
  GIT_REPOSITORY https://github.com/ggml-org/whisper.cpp.git
  GIT_TAG        v1.7.1 
)

# Disable whisper.cpp examples and tests to speed up build
set(WHISPER_BUILD_TESTS OFF CACHE BOOL "Whisper tests" FORCE)
set(WHISPER_BUILD_EXAMPLES OFF CACHE BOOL "Whisper examples" FORCE)
set(WHISPER_BUILD_SERVER OFF CACHE BOOL "Whisper server" FORCE)

FetchContent_MakeAvailable(whisper_cpp)

# 2. nlohmann/json
FetchContent_Declare(
    json
    GIT_REPOSITORY https://github.com/nlohmann/json.git
    GIT_TAG        v3.11.3
)
FetchContent_MakeAvailable(json)

# 3. miniaudio
# Download single header to build directory
set(MINIAUDIO_URL https://raw.githubusercontent.com/mackron/miniaudio/master/miniaudio.h)
set(MINIAUDIO_DIR ${CMAKE_CURRENT_BINARY_DIR}/_deps/miniaudio)
file(MAKE_DIRECTORY ${MINIAUDIO_DIR})
if(NOT EXISTS "${MINIAUDIO_DIR}/miniaudio.h")
    message(STATUS "Downloading miniaudio.h...")
    file(DOWNLOAD ${MINIAUDIO_URL} ${MINIAUDIO_DIR}/miniaudio.h)
endif()

set(WHISPER_MODELS_DIR "${CMAKE_CURRENT_SOURCE_DIR}/models")
set(WHISPER_REQUIRED_MODEL "${WHISPER_MODELS_DIR}/ggml-tiny.en.bin")
if(NOT EXISTS "${WHISPER_REQUIRED_MODEL}")
    message(FATAL_ERROR
        "Whisper model is missing: ${WHISPER_REQUIRED_MODEL}\n"
        "Run: python sidecars/whisper-stt/download_models.py"
    )
endif()

# Sources
set(SOURCES
    src/main.cpp
    src/audio.cpp
    src/inference.cpp
)

add_executable(squigit-stt ${SOURCES})

# Include directories
target_include_directories(squigit-stt PRIVATE
    ${whisper_cpp_SOURCE_DIR}             # For whisper.h in v1.7.1 (it might be in root)
    ${whisper_cpp_SOURCE_DIR}/include     # For newer versions or if moved
    ${whisper_cpp_SOURCE_DIR}/ggml/include
    ${MINIAUDIO_DIR}
)

# Link libraries
# Linking against the targets defined by FetchContent
target_link_libraries(squigit-stt PRIVATE whisper nlohmann_json::nlohmann_json)

if(APPLE)
    set_target_properties(squigit-stt PROPERTIES
        BUILD_RPATH "@loader_path/_internal"
        INSTALL_RPATH "@loader_path/_internal"
    )
elseif(UNIX)
    set_target_properties(squigit-stt PROPERTIES
        BUILD_RPATH "\$ORIGIN/_internal"
        INSTALL_RPATH "\$ORIGIN/_internal"
    )
endif()

if(UNIX AND NOT APPLE)
    target_link_libraries(squigit-stt PRIVATE pthread dl)
endif()

if(APPLE)
    target_link_libraries(squigit-stt PRIVATE "-framework CoreAudio -framework AudioToolbox -framework CoreFoundation -framework Metal -framework Foundation")
endif()
