import QtQuick

Item {
    id: root

    required property bool active
    property bool revealed: false
    property real progress: 0

    height: 70
    width: Math.min(parent.width - 32, content.implicitWidth + 56)
    x: (parent.width - width) / 2
    y: -height - 8 + (height + 8) * progress
    visible: active && progress > 0
    opacity: progress

    onActiveChanged: {
        lifetime.stop()
        revealed = false
    }

    onRevealedChanged: progress = revealed ? 1 : 0

    Behavior on progress {
        NumberAnimation {
            duration: root.active ? (root.revealed ? 280 : 200) : 0
            easing.type: Easing.OutCubic
        }
    }

    Timer {
        interval: 1
        running: root.active
        onTriggered: {
            root.revealed = true
            lifetime.restart()
        }
    }

    Timer {
        id: lifetime
        interval: 3500
        onTriggered: root.revealed = false
    }

    Rectangle {
        anchors.fill: parent
        anchors.topMargin: -14
        radius: 14
        color: "#292c30"
        border.width: 1
        border.color: "#45494f"
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
