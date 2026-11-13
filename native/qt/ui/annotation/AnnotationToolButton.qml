pragma ComponentBehavior: Bound

import QtQuick
import Qt5Compat.GraphicalEffects

Item {
    id: root
    property string iconName
    property string numberLabel: ""
    property string tip
    required property var tooltipHost
    property bool selected: false
    property bool chevron: false
    signal clicked
    signal alternateRequested
    width: 38
    height: 38
    opacity: enabled ? 1 : 0.35

    Rectangle {
        anchors.fill: parent
        radius: 7
        color: root.selected ? "#505050" : hover.containsMouse ? "#3d3d3d" : "transparent"
    }
    Image {
        id: artwork
        anchors.centerIn: parent
        width: 20
        height: 20
        source: root.numberLabel.length === 0 ? "icons/" + root.iconName + ".svg" : ""
        sourceSize: Qt.size(40, 40)
        visible: false
    }
    ColorOverlay {
        anchors.fill: artwork
        source: artwork
        color: "#f0f0f0"
        visible: root.numberLabel.length === 0
    }
    Rectangle {
        anchors.centerIn: parent
        width: 22
        height: 22
        radius: 11
        color: "transparent"
        border.color: "#f0f0f0"
        border.width: 1.5
        visible: root.numberLabel.length > 0
        Text {
            anchors.fill: parent
            anchors.margins: 3
            text: root.numberLabel
            color: "#f0f0f0"
            font.pixelSize: 12
            font.weight: Font.DemiBold
            fontSizeMode: Text.Fit
            minimumPixelSize: 7
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
        }
    }
    Canvas {
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.margins: 4
        width: 5
        height: 5
        visible: root.chevron
        onPaint: {
            const ctx = getContext("2d");
            ctx.clearRect(0, 0, width, height);
            ctx.fillStyle = "#cccccc";
            ctx.beginPath();
            ctx.moveTo(0, 5);
            ctx.lineTo(5, 0);
            ctx.lineTo(5, 5);
            ctx.closePath();
            ctx.fill();
        }
    }
    MouseArea {
        id: hover
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        hoverEnabled: true
        onEntered: root.tooltipHost.showTip(root)
        onExited: root.tooltipHost.leaveTip(root)
        onPressed: root.tooltipHost.dismissTips()
        onClicked: function (mouse) {
            if (mouse.button === Qt.RightButton)
                root.alternateRequested();
            else
                root.clicked();
        }
    }
}
