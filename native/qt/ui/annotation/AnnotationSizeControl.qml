pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls

Item {
    id: root
    required property var document
    property int orientation: Qt.Horizontal
    readonly property bool vertical: orientation === Qt.Vertical
    implicitWidth: vertical ? 36 : 218
    implicitHeight: vertical ? 212 : 32

    Slider {
        id: slider
        width: root.vertical ? root.width : root.width - 40
        height: root.vertical ? root.height - 26 : root.height
        padding: 0
        orientation: root.orientation
        from: 1
        to: 160
        stepSize: 1
        value: root.document ? root.document.toolSize : 1
        onMoved: root.document.toolSize = value
        background: Rectangle {
            x: root.vertical ? (slider.width - width) / 2 : 0
            y: root.vertical ? 0 : (slider.height - height) / 2
            width: root.vertical ? 5 : slider.width
            height: root.vertical ? slider.height : 5
            radius: 2.5
            color: "#f0f0f0"
            border.color: "#555555"
        }
        handle: Rectangle {
            x: root.vertical ? (slider.width - width) / 2 : slider.visualPosition * (slider.width - width)
            y: root.vertical ? slider.visualPosition * (slider.height - height) : (slider.height - height) / 2
            width: 16
            height: 16
            radius: 8
            color: "#f5f5f5"
            border.color: "#555555"
        }
    }
    Text {
        x: root.vertical ? 0 : slider.width + 8
        y: root.vertical ? slider.height + 6 : (root.height - height) / 2
        width: root.vertical ? root.width : 32
        horizontalAlignment: Text.AlignHCenter
        text: root.document ? Math.round(root.document.toolSize) : ""
        color: "white"
        style: Text.Outline
        styleColor: "#333333"
        font.pixelSize: 11
    }
}
