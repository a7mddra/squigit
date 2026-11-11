import QtQuick

Item {
    id: root

    required property bool active
    readonly property int topInset: 64
    property bool hintRevealed: false
    property bool exitVisible: false
    property bool exitHovered: edgeSensor.containsMouse || exitMouse.containsMouse

    signal exitRequested()

    visible: active

    onActiveChanged: {
        lifetime.stop()
        hideTimer.stop()
        hintRevealed = false
        exitVisible = false
    }

    onExitHoveredChanged: {
        if (exitHovered) {
            hideTimer.stop()
            exitVisible = true
        } else {
            hideTimer.restart()
        }
    }

    Timer {
        id: startupTimer
        interval: 1
        running: root.active
        onTriggered: {
            root.hintRevealed = true
            lifetime.restart()
        }
    }

    Timer {
        id: lifetime
        interval: 3500
        onTriggered: root.hintRevealed = false
    }

    Timer {
        id: hideTimer
        interval: 500
        onTriggered: root.exitVisible = false
    }

    MouseArea {
        id: edgeSensor
        width: Math.min(parent.width, 320)
        height: 24
        anchors.top: parent.top
        anchors.horizontalCenter: parent.horizontalCenter
        hoverEnabled: true
        acceptedButtons: Qt.NoButton
    }

    Rectangle {
        id: hintPill
        y: root.topInset
        width: Math.min(parent.width - 32, content.implicitWidth + 56)
        height: 44
        anchors.horizontalCenter: parent.horizontalCenter
        radius: height / 2
        color: "#292c30"
        border.width: 1
        border.color: "#45494f"
        opacity: root.hintRevealed && !root.exitVisible ? 1 : 0
        visible: opacity > 0

        Behavior on opacity {
            NumberAnimation { duration: 180; easing.type: Easing.OutCubic }
        }

        Row {
            id: content
            anchors.centerIn: parent
            spacing: 10

            Text {
                anchors.verticalCenter: parent.verticalCenter
                text: "To exit, press and hold"
                color: "#f3f4f5"
                font.pixelSize: 16
            }

            Rectangle {
                width: 44
                height: 32
                radius: 4
                color: "#35393f"
                border.width: 1
                border.color: "#aeb4bd"

                Text {
                    anchors.centerIn: parent
                    text: "Esc"
                    color: "#ffffff"
                    font.pixelSize: 15
                }
            }
        }
    }

    Rectangle {
        id: exitButton
        y: root.topInset
        width: 44
        height: 44
        radius: width / 2
        anchors.horizontalCenter: parent.horizontalCenter
        color: exitMouse.containsMouse ? "#3a3d42" : "#292c30"
        border.width: 1
        border.color: "#45494f"
        opacity: root.exitVisible ? 1 : 0
        visible: opacity > 0

        Behavior on opacity {
            NumberAnimation { duration: 160; easing.type: Easing.OutCubic }
        }

        Canvas {
            anchors.centerIn: parent
            width: 16
            height: 16

            onPaint: {
                var ctx = getContext("2d")
                ctx.clearRect(0, 0, width, height)
                ctx.strokeStyle = "#ffffff"
                ctx.lineWidth = 2.0
                ctx.lineCap = "round"
                ctx.beginPath()
                ctx.moveTo(4, 4)
                ctx.lineTo(12, 12)
                ctx.moveTo(12, 4)
                ctx.lineTo(4, 12)
                ctx.stroke()
            }
        }

        MouseArea {
            id: exitMouse
            anchors.fill: parent
            enabled: root.exitVisible
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: root.exitRequested()
        }
    }
}
