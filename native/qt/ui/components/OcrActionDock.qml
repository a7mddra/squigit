pragma ComponentBehavior: Bound

import QtQuick

FloatingCapsule {
    id: root
    required property var controller
    property bool hasTextSelection: false
    signal actionRequested(string action)

    Flickable {
        id: viewport
        anchors.fill: parent
        anchors.margins: 8
        contentWidth: Math.max(width, actions.width)
        contentHeight: height
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        flickableDirection: Flickable.HorizontalFlick

        Row {
            id: actions
            x: Math.max(0, (viewport.width - width) / 2)
            height: viewport.height
            Repeater {
                model: [{
                        "label": "Select all",
                        "action": "selectAll",
                        "active": root.controller.ocrVisible && root.controller.hasOcrText
                    }, {
                        "label": "Copy",
                        "action": "copy",
                        "active": root.hasTextSelection
                    }, {
                        "label": "Search",
                        "action": "search",
                        "active": root.hasTextSelection
                    }, {
                        "label": "Open in Translate",
                        "action": "translate",
                        "active": root.hasTextSelection
                    }, {
                        "label": root.controller.ocrVisible ? "Hide OCR" : "Show OCR",
                        "action": "visibility",
                        "active": true
                    }]
                delegate: Item {
                    id: button
                    required property var modelData
                    required property int index
                    width: label.implicitWidth + 28
                    height: viewport.height

                    Rectangle {
                        anchors.fill: parent
                        anchors.margins: 2
                        radius: 8
                        color: buttonMouse.pressed ? "#20000000" : buttonMouse.containsMouse ? "#10000000" : "transparent"
                    }
                    Text {
                        id: label
                        anchors.centerIn: parent
                        text: button.modelData.label
                        color: "#1c1c1e"
                        opacity: button.modelData.active ? 1 : 0.35
                        font.family: Qt.platform.os === "osx" ? "SF Pro Text" : "Segoe UI"
                        font.pixelSize: 14
                    }
                    Rectangle {
                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        width: 1
                        height: 18
                        color: "#24000000"
                        visible: button.index < 4
                    }
                    MouseArea {
                        id: buttonMouse
                        anchors.fill: parent
                        enabled: button.modelData.active
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.actionRequested(button.modelData.action)
                    }
                }
            }
        }
    }
}
