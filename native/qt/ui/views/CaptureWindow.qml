pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Window
import "../components"

Window {
    id: root

    flags: Qt.FramelessWindowHint | Qt.WindowStaysOnTopHint | Qt.BypassWindowManagerHint | Qt.Tool

    x: Screen.virtualX
    y: Screen.virtualY
    width: Screen.width
    height: Screen.height

    color: "transparent"

    required property var controller
    property bool ownsInput: false

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
        }

        Rectangle {
            id: toolbar
            x: Math.max(8, Math.min(parent.area.x, root.width - width - 8))
            y: Math.max(8, Math.min(parent.area.y + parent.area.height + 10, root.height - height - 8))
            width: actions.width + 12
            height: 44
            radius: 6
            color: "#202124"
            Row {
                id: actions
                anchors.centerIn: parent
                spacing: 6
                Repeater {
                    model: root.controller.ocrPending ? ["Save", "Cancel OCR"] : ["Save", "OCR"]
                    delegate: Rectangle {
                        id: actionButton
                        required property string modelData
                        readonly property bool checked: modelData === "OCR" && root.controller.ocrVisible
                        width: modelData === "Cancel OCR" ? 110 : 76
                        height: 32
                        radius: 4
                        color: checked ? "#356ab0" : actionMouse.containsMouse ? "#505154" : "#353639"
                        opacity: modelData === "Cancel OCR" && root.controller.ocrCancelling ? 0.5 : 1
                        Text {
                            anchors.centerIn: parent
                            text: parent.modelData
                            color: "white"
                            font.pixelSize: 14
                        }
                        MouseArea {
                            id: actionMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            enabled: actionButton.modelData !== "Cancel OCR" || !root.controller.ocrCancelling
                            onClicked: {
                                if (actionButton.modelData === "Save")
                                    root.controller.save();
                                else if (actionButton.modelData === "Cancel OCR")
                                    root.controller.cancelOcr();
                                else
                                    root.controller.toggleOcr();
                            }
                        }
                    }
                }
            }
        }

        Rectangle {
            visible: root.controller.status.length > 0
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
                text: root.controller.status
                wrapMode: Text.Wrap
                font.pixelSize: 13
                color: "white"
            }
        }
    }

    Shortcut {
        sequence: "Escape"
        autoRepeat: false
        context: Qt.ApplicationShortcut
        enabled: root.ownsInput && root.visible
        onActivated: {
            if (!selectionCrop.clearTextSelection())
                root.controller.cancel();
        }
    }

    Shortcut {
        sequence: "Q"
        context: Qt.ApplicationShortcut
        enabled: root.ownsInput && root.visible
        onActivated: root.controller.cancel()
    }

    Shortcut {
        sequences: ["Ctrl+Shift+C", "Meta+Shift+C"]
        context: Qt.ApplicationShortcut
        enabled: root.ownsInput && root.visible && root.controller.hasSelection
        onActivated: root.controller.copyText()
    }
}
