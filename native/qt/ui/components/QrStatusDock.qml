pragma ComponentBehavior: Bound

import QtQuick

FloatingCapsule {
    id: root
    required property var controller
    signal askGoogleRequested

    height: Math.max(56, statusContent.implicitHeight + 24)

    Row {
        id: statusContent
        anchors.centerIn: parent
        spacing: 12

        Text {
            id: statusText
            anchors.verticalCenter: parent.verticalCenter
            width: Math.min(implicitWidth, Math.max(1, root.width - 48 - (fallbackButton.visible ? fallbackButton.width + statusContent.spacing : 0)))
            text: root.controller.qrStatus
            textFormat: Text.PlainText
            wrapMode: Text.Wrap
            horizontalAlignment: Text.AlignHCenter
            font.pixelSize: 14
            color: "#1c1c1e"
        }

        Rectangle {
            id: fallbackButton
            anchors.verticalCenter: parent.verticalCenter
            visible: root.controller.qrFallbackAvailable
            width: fallbackLabel.implicitWidth + 20
            height: 32
            radius: 8
            color: fallbackMouse.pressed ? "#20000000" : fallbackMouse.containsMouse ? "#10000000" : "transparent"

            Row {
                id: fallbackLabel
                anchors.centerIn: parent
                spacing: 6

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: "Ask Google?"
                    color: statusText.color
                    font: statusText.font
                }

                Image {
                    anchors.verticalCenter: parent.verticalCenter
                    width: 16
                    height: 16
                    source: "icons/open-external.svg"
                    sourceSize: Qt.size(32, 32)
                }
            }

            MouseArea {
                id: fallbackMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: root.askGoogleRequested()
            }
        }
    }
}
