pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Window
import "../components"

Window {
    id: root

    flags: Qt.FramelessWindowHint | Qt.WindowStaysOnTopHint | Qt.Tool

    x: Screen.virtualX
    y: Screen.virtualY
    width: Screen.width
    height: Screen.height

    onClosing: function(close) {
        close.accepted = false;
        root.controller.cancel();
    }

    color: "transparent"

    required property var controller
    property bool ownsInput: false
    readonly property bool saveMode: controller.action === "snapshot"

    function escapeTapped() {
        if (!selectionCrop.clearTextSelection())
            root.controller.cancel();
    }

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
                const fallbackSource = root.controller.ensureBackgroundFileFallback();
                if (fallbackSource && source !== fallbackSource) {
                    source = fallbackSource;
                }
            }
        }
    }

    Rectangle {
        id: dimOverlay
        anchors.fill: parent
        visible: !root.controller.hasSelection && root.controller.captureMode !== "traditional"

        gradient: Gradient {
            GradientStop {
                position: 0.0
                color: Qt.rgba(0, 0, 0, 0.5)
            }
            GradientStop {
                position: 1.0
                color: "transparent"
            }
        }

        NumberAnimation on opacity  {
            from: 0
            to: 1
            duration: 200
            running: dimOverlay.visible
            easing.type: Easing.OutQuad
        }
    }

    Loader {
        id: canvasLoader
        anchors.fill: parent
        focus: true
        visible: !root.controller.hasSelection

        source: root.controller.captureMode === "traditional" ? "../components/TraditionalCanvas.qml" : "../components/SquiggleCanvas.qml"

        onLoaded: {
            if (item) {
                item.controller = root.controller;
                item.forceActiveFocus();
            }
        }
    }

    FocusScope {
        id: selectionStage
        anchors.fill: parent
        visible: root.controller.hasSelection
        focus: visible
        property rect area: root.controller.selection

        onVisibleChanged: {
            if (visible)
                forceActiveFocus();
        }

        SelectionCrop {
            id: selectionCrop
            controller: root.controller
            visible: root.controller.hasSelection
            enabled: !root.controller.qrPending
        }

        Rectangle {
            id: toolbar
            x: Math.max(8, Math.min(parent.area.x, root.width - width - 8))
            y: Math.max(8, Math.min(parent.area.y + parent.area.height + 10, root.height - height - 8))
            width: (root.saveMode ? saveButton.width : visibilityButton.width) + 12
            height: 44
            radius: 6
            color: "#202124"
            visible: root.saveMode || root.controller.action === "ocr"
            Rectangle {
                id: saveButton
                anchors.centerIn: parent
                width: 88
                height: 32
                radius: 4
                color: saveMouse.containsMouse ? "#505154" : "#353639"
                visible: root.saveMode
                Text {
                    anchors.centerIn: parent
                    text: "Save"
                    color: "white"
                    font.pixelSize: 14
                }
                MouseArea {
                    id: saveMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.controller.save()
                }
            }
            Rectangle {
                id: visibilityButton
                anchors.centerIn: parent
                visible: root.controller.action === "ocr"
                width: 132
                height: 32
                radius: 4
                color: actionMouse.containsMouse ? "#505154" : "#353639"
                opacity: root.controller.ocrCancelling ? 0.5 : 1
                Text {
                    anchors.centerIn: parent
                    text: root.controller.ocrCancelling ? "Stopping OCR..."
                        : root.controller.ocrPending ? "Cancel OCR"
                        : !root.controller.ocrReady ? (root.controller.ocrFailed ? "Retry OCR" : "Scan OCR")
                        : root.controller.ocrVisible ? "Hide OCR" : "Show OCR"
                    color: "white"
                    font.pixelSize: 14
                }
                MouseArea {
                    id: actionMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    enabled: !root.controller.ocrCancelling
                    onClicked: {
                        if (root.controller.ocrPending)
                            root.controller.cancelOcr();
                        else
                            root.controller.toggleOcr();
                    }
                }
            }
        }

        Rectangle {
            visible: statusText.text.length > 0
            x: Math.max(8, Math.min(toolbar.x, root.width - width - 8))
            y: toolbar.y >= height + 18 ? toolbar.y - height - 8 : toolbar.y + toolbar.height + 8
            width: Math.min(760, root.width - 16)
            height: statusText.implicitHeight + 16
            radius: 4
            color: "#202124"
            Text {
                id: statusText
                anchors.fill: parent
                anchors.margins: 8
                text: root.controller.qrStatus.length > 0 ? root.controller.qrStatus : root.controller.status
                wrapMode: Text.Wrap
                font.pixelSize: 13
                color: "white"
            }
        }

        AskGoogleComposer {
            id: askComposer
            visible: root.controller.action === "ask-google" && root.controller.hasSelection
            x: Math.max(8, (root.width - width) / 2)
            y: Math.max(8, root.height - height - 48)
            onSent: function(text) {
                root.controller.askGoogle(text)
            }
        }
    }

    CaptureExitHint {
        anchors.fill: parent
        active: root.visible
        z: 100
        onExitRequested: root.controller.cancel()
    }

    Shortcut {
        sequence: "Q"
        context: Qt.ApplicationShortcut
        enabled: root.ownsInput && root.visible && !askComposer.visible
        onActivated: root.controller.cancel()
    }

    Shortcut {
        sequences: ["Ctrl+C", "Ctrl+Insert"]
        context: Qt.ApplicationShortcut
        enabled: root.ownsInput && root.visible && root.controller.ocrVisible
        onActivated: selectionCrop.copyTextSelection()
    }

    Shortcut {
        sequences: ["Ctrl+Shift+C", "Meta+Shift+C"]
        context: Qt.ApplicationShortcut
        enabled: root.ownsInput && root.visible && root.controller.hasSelection && !askComposer.visible
        onActivated: root.controller.copyText()
    }
}
