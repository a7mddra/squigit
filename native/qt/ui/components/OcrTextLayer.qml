import QtQuick

Loader {
    id: root
    anchors.fill: parent

    required property var controller
    property bool interacting: false
    active: controller.ocrVisible
    focus: active

    sourceComponent: Item {
        anchors.fill: parent

        Repeater {
            model: root.controller.textBoxes

            delegate: Item {
                id: line
                required property var modelData
                readonly property var box: modelData.box
                readonly property real ratio: root.controller.pixelRatio

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
                    color: "transparent"
                    selectedTextColor: "transparent"
                    selectionColor: "#805daaff"
                    transform: Scale {
                        xScale: line.width / selectableLine.width
                    }
                }
            }
        }
    }
}
