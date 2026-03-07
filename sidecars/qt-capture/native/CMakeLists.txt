cmake_minimum_required(VERSION 3.16)

if(APPLE)
    project(CAPTURE VERSION 0.1.0 LANGUAGES CXX OBJCXX)
else()
    project(CAPTURE VERSION 0.1.0 LANGUAGES CXX)
endif()

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)

cmake_policy(SET CMP0071 NEW)
if(POLICY CMP0169)
    cmake_policy(SET CMP0169 OLD)
endif()

find_package(Qt6 REQUIRED COMPONENTS Core Gui)
find_package(Qt6 REQUIRED COMPONENTS Quick)
find_package(Qt6 REQUIRED COMPONENTS Qml)
if(UNIX AND NOT APPLE)
    find_package(Qt6 REQUIRED COMPONENTS DBus)
endif()

configure_file(
    "${CMAKE_CURRENT_SOURCE_DIR}/cmake/config.h.in"
    "${CMAKE_CURRENT_BINARY_DIR}/generated/config.h"
    @ONLY
)
include_directories("${CMAKE_CURRENT_BINARY_DIR}/generated")

set(SOURCES 
    src/main.cpp 
    src/core/ScreenGrabber.h
    src/core/CaptureMode.h
    src/controller/CaptureController.cpp
    src/controller/CaptureController.h
)

if(WIN32)
    list(APPEND SOURCES src/grabber/GrabberWin.cpp)
    set(PLATFORM_LIBS dwmapi gdiplus user32 gdi32 Shcore)
elseif(APPLE)
    list(APPEND SOURCES src/grabber/GrabberMac.mm)
    
    find_library(FOUNDATION_LIB Foundation)
    find_library(COREGRAPHICS_LIB CoreGraphics)
    find_library(COCOA_LIB Cocoa)
    find_library(APPKIT_LIB AppKit)
    
    if(NOT FOUNDATION_LIB OR NOT COREGRAPHICS_LIB OR NOT COCOA_LIB OR NOT APPKIT_LIB)
        message(FATAL_ERROR "Required macOS frameworks not found")
    endif()
    set(PLATFORM_LIBS ${FOUNDATION_LIB} ${COREGRAPHICS_LIB} ${COCOA_LIB} ${APPKIT_LIB})
elseif(UNIX AND NOT APPLE)
    list(APPEND SOURCES src/grabber/GrabberLinux.cpp)
    set(PLATFORM_LIBS Qt6::DBus)
endif()

qt_add_executable(capture WIN32 MACOSX_BUNDLE ${SOURCES})

qt_add_qml_module(capture
    URI CaptureQml
    VERSION 1.0
    QML_FILES
        qml/CaptureWindow.qml
        qml/SquiggleCanvas.qml
        qml/RectangleCanvas.qml
)

target_include_directories(capture PRIVATE 
    src/core
    src/grabber
    src/controller
)

target_link_libraries(capture PRIVATE 
    Qt6::Core Qt6::Gui 
    Qt6::Quick Qt6::Qml
    ${PLATFORM_LIBS}
)

if(UNIX AND NOT APPLE)
    set_target_properties(capture PROPERTIES
        OUTPUT_NAME "capture-bin"
        INSTALL_RPATH "$ORIGIN/../lib" 
        BUILD_WITH_INSTALL_RPATH TRUE
    )
endif()

if(APPLE)
    set_target_properties(capture PROPERTIES
        MACOSX_BUNDLE TRUE
        MACOSX_BUNDLE_GUI_IDENTIFIER "com.capture.app"
        MACOSX_BUNDLE_BUNDLE_NAME "Capture"
        MACOSX_BUNDLE_INFO_STRING "Capture Screen Capture"
        MACOSX_BUNDLE_INFO_PLIST "${CMAKE_CURRENT_SOURCE_DIR}/Info.plist.in"
    )
endif()
