import QtQuick
import QtQuick.Window

/**
 * Main capture overlay window.
 *
 * This window displays fullscreen over a single monitor, showing the frozen
 * screenshot as background with either squiggle or traditional selection mode.
 *
 * Critical window flags ensure instant appearance without OS animations:
 * - Qt.FramelessWindowHint: No title bar or borders
 * - Qt.WindowStaysOnTopHint: Always on top
 * - Qt.BypassWindowManagerHint: Bypass WM completely (critical for GNOME top bar)
 * - Qt.Tool: Skips taskbar/dock, treated as utility window
 */

Window {
    id: root

    flags: Qt.FramelessWindowHint
           | Qt.WindowStaysOnTopHint
           | Qt.BypassWindowManagerHint
           | Qt.Tool

    x: Screen.virtualX
    y: Screen.virtualY
    width: Screen.width
    height: Screen.height

    color: "transparent"

    required property var controller

    Image {
        id: background
        anchors.fill: parent
        source: root.controller.backgroundSource
        fillMode: Image.Stretch
        smooth: false
        mipmap: false
        asynchronous: false
        cache: false

        onStatusChanged: {
            if (status === Image.Error) {
                const fallbackSource = root.controller.ensureBackgroundFileFallback()
                if (fallbackSource && source !== fallbackSource) {
                    source = fallbackSource
                }
            }
        }
    }

    Rectangle {
        id: dimOverlay
        anchors.fill: parent
        opacity: 0
        visible: root.controller.captureMode !== "traditional"

        gradient: Gradient {
            GradientStop { position: 0.0; color: Qt.rgba(0, 0, 0, 0.5) }
            GradientStop { position: 1.0; color: "transparent" }
        }

        NumberAnimation on opacity {
            from: 0; to: 1
            duration: 200
            running: dimOverlay.visible
            easing.type: Easing.OutQuad
        }
    }

    Loader {
        id: canvasLoader
        anchors.fill: parent
        focus: true

        source: root.controller.captureMode === "traditional"
            ? "../components/TraditionalCanvas.qml"
            : "../components/SquiggleCanvas.qml"

        onLoaded: {
            if (item) {
                item.controller = root.controller
                item.forceActiveFocus()
            }
        }
    }

    Shortcut {
        sequence: "Escape"
        onActivated: root.controller.cancel()
    }

    Shortcut {
        sequence: "Q"
        onActivated: root.controller.cancel()
    }

}
