cmake_minimum_required(VERSION 3.16)

if(APPLE)
    if(NOT DEFINED CMAKE_OSX_DEPLOYMENT_TARGET)
        set(CMAKE_OSX_DEPLOYMENT_TARGET "11.0" CACHE STRING "Minimum macOS version")
    endif()
    project(SQUIGIT VERSION 0.1.0 LANGUAGES CXX OBJCXX)
else()
    project(SQUIGIT VERSION 0.1.0 LANGUAGES CXX)
endif()

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)

cmake_policy(SET CMP0071 NEW)
if(POLICY CMP0169)
    cmake_policy(SET CMP0169 OLD)
endif()

find_package(Qt6 6.4 REQUIRED COMPONENTS Core Gui)
find_package(Qt6 6.4 REQUIRED COMPONENTS Quick)
find_package(Qt6 6.4 REQUIRED COMPONENTS Qml)
if(UNIX AND NOT APPLE)
    find_package(Qt6 6.4 REQUIRED COMPONENTS DBus)
endif()

if(COMMAND qt_policy)
    qt_policy(SET QTP0001 OLD)
endif()

configure_file(
    "${CMAKE_CURRENT_SOURCE_DIR}/config/config.h.in"
    "${CMAKE_CURRENT_BINARY_DIR}/generated/config.h"
    @ONLY
)
include_directories("${CMAKE_CURRENT_BINARY_DIR}/generated")

set(SOURCES
    src/main.cpp
    src/CaptureEngine.cpp
    src/CaptureEngine.h
    src/menu/RadialMenu.cpp
    src/menu/RadialMenu.h
    src/menu/RadialPetal.cpp
    src/menu/RadialPetal.h
    src/menu/RadialGeometry.h
    src/actions/Actions.h
    src/actions/Actions.cpp
    src/actions/CircleToSearch.cpp
    src/actions/Snapshot.cpp
    src/actions/ScanQr.cpp
    src/actions/Ocr.cpp
    src/actions/AskGoogle.cpp
    backends/include/Grabber.h
    ui/qml.qrc
)

if(WIN32)
    list(APPEND SOURCES
        backends/windows/GrabberWin.cpp
        backends/windows/GrabberWin.h
    )
    set(PLATFORM_LIBS dwmapi gdiplus user32 gdi32 Shcore)
elseif(APPLE)
    list(APPEND SOURCES
        backends/macos/GrabberMac.mm
        backends/macos/GrabberMac.h
    )

    find_library(FOUNDATION_LIB Foundation)
    find_library(COREGRAPHICS_LIB CoreGraphics)
    find_library(COCOA_LIB Cocoa)
    find_library(APPKIT_LIB AppKit)

    if(NOT FOUNDATION_LIB OR NOT COREGRAPHICS_LIB OR NOT COCOA_LIB OR NOT APPKIT_LIB)
        message(FATAL_ERROR "Required macOS frameworks not found")
    endif()
    set(PLATFORM_LIBS ${FOUNDATION_LIB} ${COREGRAPHICS_LIB} ${COCOA_LIB} ${APPKIT_LIB})
elseif(UNIX AND NOT APPLE)
    list(APPEND SOURCES
        backends/linux/GrabberLinux.cpp
        backends/linux/GrabberLinux.h
    )
    set(PLATFORM_LIBS Qt6::DBus)
endif()

qt_add_executable(squigit-qt WIN32 MACOSX_BUNDLE ${SOURCES})

qt_add_qml_module(squigit-qt
    URI SquigitCapture
    VERSION 1.0
)

target_include_directories(squigit-qt PRIVATE
    src
    backends/include
    backends/linux
    backends/macos
    backends/windows
)

target_link_libraries(squigit-qt PRIVATE
    Qt6::Core Qt6::Gui
    Qt6::Quick Qt6::Qml
    ${PLATFORM_LIBS}
)

if(UNIX AND NOT APPLE)
    set_target_properties(squigit-qt PROPERTIES
        OUTPUT_NAME "squigit-qt"
        INSTALL_RPATH "$ORIGIN/../lib"
        BUILD_WITH_INSTALL_RPATH TRUE
    )
endif()

if(APPLE)
    set_target_properties(squigit-qt PROPERTIES
        OUTPUT_NAME "squigit-qt"
        MACOSX_BUNDLE TRUE
        MACOSX_BUNDLE_GUI_IDENTIFIER "io.github.a7mddra.squigit.qt"
        MACOSX_BUNDLE_BUNDLE_NAME "squigit-qt"
        MACOSX_BUNDLE_INFO_STRING "Squigit Screen Capture"
        MACOSX_BUNDLE_INFO_PLIST "${CMAKE_CURRENT_SOURCE_DIR}/config/Info.plist.in"
    )
endif()
