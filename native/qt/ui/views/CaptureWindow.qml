import QtQuick
import QtQuick.Window

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
        visible: !root.controller.hasSelection && root.controller.captureMode !== "traditional"

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
        visible: !root.controller.hasSelection

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

    Item {
        anchors.fill: parent
        visible: root.controller.hasSelection
        property rect area: root.controller.selection

        Rectangle { x: 0; y: 0; width: parent.width; height: parent.area.y; color: "#70000000" }
        Rectangle { x: 0; y: parent.area.y + parent.area.height; width: parent.width; height: parent.height - y; color: "#70000000" }
        Rectangle { x: 0; y: parent.area.y; width: parent.area.x; height: parent.area.height; color: "#70000000" }
        Rectangle { x: parent.area.x + parent.area.width; y: parent.area.y; width: parent.width - x; height: parent.area.height; color: "#70000000" }
        Rectangle {
            x: parent.area.x; y: parent.area.y
            width: parent.area.width; height: parent.area.height
            color: "transparent"
            border.color: "white"
            border.width: 1
        }

        Repeater {
            model: root.controller.textBoxes
            delegate: Item {
                required property var modelData
                readonly property var box: modelData.box
                readonly property real ratio: root.controller.pixelRatio
                x: root.controller.selection.x + box[0][0] / ratio
                y: root.controller.selection.y + box[0][1] / ratio
                width: Math.hypot(box[1][0] - box[0][0], box[1][1] - box[0][1]) / ratio
                height: Math.hypot(box[3][0] - box[0][0], box[3][1] - box[0][1]) / ratio
                rotation: Math.atan2(box[1][1] - box[0][1], box[1][0] - box[0][0]) * 180 / Math.PI
                transformOrigin: Item.TopLeft

                Rectangle { anchors.fill: parent; color: "#185daaff" }
                TextEdit {
                    id: selectableLine
                    text: modelData.text
                    width: Math.max(1, implicitWidth)
                    height: parent.height
                    font.pixelSize: Math.max(1, parent.height * 0.85)
                    textFormat: TextEdit.PlainText
                    wrapMode: TextEdit.NoWrap
                    readOnly: true
                    selectByMouse: true
                    color: "transparent"
                    selectedTextColor: "transparent"
                    selectionColor: "#805daaff"
                    transform: Scale { xScale: selectableLine.parent.width / selectableLine.width }
                }
            }
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
                    model: ["Save", "OCR"]
                    delegate: Rectangle {
                        required property string modelData
                        width: 76; height: 32; radius: 4
                        color: actionMouse.containsMouse ? "#505154" : "#353639"
                        opacity: modelData === "OCR" && root.controller.ocrPending ? 0.5 : 1
                        Text { anchors.centerIn: parent; text: parent.modelData; color: "white"; font.pixelSize: 14 }
                        MouseArea {
                            id: actionMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            enabled: modelData !== "OCR" || !root.controller.ocrPending
                            onClicked: modelData === "Save" ? root.controller.save() : root.controller.recognize()
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
        sequence: "Ctrl+Shift+C"
        onActivated: root.controller.copyText()
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
