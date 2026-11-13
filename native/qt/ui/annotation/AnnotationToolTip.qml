pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls

ToolTip {
    id: root
    z: 300
    delay: 550
    timeout: 4500
    padding: 8
    leftPadding: 10
    rightPadding: 10
    y: -height - 10
    enter: Transition {
    }
    exit: Transition {
    }
    background: Rectangle {
        color: "#242424"
        radius: 6
        border.color: "#454545"
    }
    contentItem: Text {
        text: root.text
        color: "#f5f5f5"
        font.pixelSize: 12
    }
}
