pragma ComponentBehavior: Bound

import QtQuick

FocusScope {
    id: root
    anchors.fill: parent

    required property var controller
    property bool interacting: false
    visible: controller.ocrVisible
    enabled: visible && !interacting
    focus: visible

    function clearSelection(exceptLine) {
        let selected = false;
        for (let i = 0; i < textLines.count; ++i) {
            const line = textLines.itemAt(i);
            if (line && line !== exceptLine)
                selected = line.clearSelection() || selected;
        }
        if (visible && enabled)
            forceActiveFocus();
        return selected;
    }

    function copySelection() {
        for (let i = 0; i < textLines.count; ++i) {
            const line = textLines.itemAt(i);
            if (line && line.copySelection())
                return;
        }
    }

    onInteractingChanged: {
        if (interacting)
            clearSelection();
        else if (visible)
            forceActiveFocus();
    }

    onVisibleChanged: {
        if (!visible)
            clearSelection();
    }

    MouseArea {
        anchors.fill: parent
        onPressed: function (mouse) {
            root.clearSelection();
            mouse.accepted = false;
        }
    }

    Repeater {
        id: textLines
        model: root.controller.ocrVisible ? root.controller.textBoxes : []

        delegate: Item {
            id: line
            required property var modelData
            readonly property var box: modelData.box
            readonly property real ratio: root.controller.pixelRatio

            function clearSelection() {
                const selected = selectableLine.selectedText.length > 0;
                selectableLine.deselect();
                return selected;
            }

            function copySelection() {
                if (selectableLine.selectedText.length === 0)
                    return false;
                selectableLine.copy();
                return true;
            }

            x: box[0][0] / ratio
            y: box[0][1] / ratio
            width: Math.hypot(box[1][0] - box[0][0], box[1][1] - box[0][1]) / ratio
            height: Math.hypot(box[3][0] - box[0][0], box[3][1] - box[0][1]) / ratio
            rotation: Math.atan2(box[1][1] - box[0][1], box[1][0] - box[0][0]) * 180 / Math.PI
            transformOrigin: Item.TopLeft

            Rectangle {
                anchors.fill: parent
                color: line.modelData.inside ? "#185daaff" : "#40969aa0"
            }

            TextEdit {
                id: selectableLine
                text: line.modelData.text
                width: Math.max(1, implicitWidth)
                height: line.height
                font.pixelSize: Math.max(1, line.height * 0.85)
                textFormat: TextEdit.PlainText
                wrapMode: TextEdit.NoWrap
                readOnly: true
                visible: line.modelData.inside
                enabled: line.modelData.inside && !root.interacting
                selectByMouse: true
                activeFocusOnPress: false
                persistentSelection: true
                color: "transparent"
                selectedTextColor: "transparent"
                selectionColor: "#805daaff"
                transform: Scale {
                    xScale: line.width / selectableLine.width
                }

                PointHandler {
                    acceptedButtons: Qt.LeftButton
                    onActiveChanged: {
                        if (active)
                            root.clearSelection(line);
                    }
                }
            }
        }
    }
}
