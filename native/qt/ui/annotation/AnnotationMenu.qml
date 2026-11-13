pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import Qt5Compat.GraphicalEffects

Popup {
    id: root
    property var entries: []
    property bool aboveAnchor: false
    signal triggered(string action)
    width: 234
    z: 200
    padding: 6
    focus: true
    modal: false
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

    function showAt(px, py, items) {
        if (visible)
            close();
        entries = items;
        x = Math.max(8, Math.min(px, parent.width - width - 8));
        const menuHeight = items.reduce(function (total, entry) {
                return total + (entry.separator ? 11 : 34);
            }, 10);
        const top = aboveAnchor ? py - menuHeight : py;
        y = Math.max(8, Math.min(top, parent.height - menuHeight - 8));
        open();
    }

    background: Rectangle {
        color: "#262626"
        radius: 9
        border.color: "#454545"
        layer.enabled: true
        layer.effect: DropShadow {
            horizontalOffset: 0
            verticalOffset: 5
            radius: 18
            samples: 37
            color: "#70000000"
        }
    }

    contentItem: Column {
        spacing: 2
        Repeater {
            model: root.entries
            delegate: Item {
                id: entry
                required property var modelData
                width: root.availableWidth
                height: modelData.separator ? 9 : 32
                enabled: modelData.enabled !== false

                Rectangle {
                    anchors.centerIn: parent
                    width: parent.width - 12
                    height: 1
                    color: "#484848"
                    visible: entry.modelData.separator === true
                }
                Rectangle {
                    anchors.fill: parent
                    visible: !entry.modelData.separator
                    color: hover.containsMouse && entry.enabled ? "#4a4a4a" : entry.modelData.checked ? "#3a3a3a" : "transparent"
                    radius: 5
                    Image {
                        x: 8
                        anchors.verticalCenter: parent.verticalCenter
                        width: 14
                        height: 14
                        source: "icons/check.svg"
                        sourceSize: Qt.size(28, 28)
                        visible: entry.modelData.checked === true
                    }
                    Text {
                        anchors.left: parent.left
                        anchors.leftMargin: 30
                        anchors.verticalCenter: parent.verticalCenter
                        text: entry.modelData.label || ""
                        color: entry.enabled ? "#f5f5f5" : "#858585"
                        font.pixelSize: 12
                    }
                    Text {
                        anchors.right: parent.right
                        anchors.rightMargin: 10
                        anchors.verticalCenter: parent.verticalCenter
                        text: entry.modelData.shortcut || ""
                        color: "#adadad"
                        font.pixelSize: 11
                    }
                    MouseArea {
                        id: hover
                        anchors.fill: parent
                        hoverEnabled: true
                        enabled: entry.enabled && !entry.modelData.separator
                        onClicked: {
                            root.close();
                            root.triggered(entry.modelData.action);
                        }
                    }
                }
            }
        }
    }
}
