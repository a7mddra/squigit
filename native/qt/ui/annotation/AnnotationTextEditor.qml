pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: root
    required property var document
    property real pixelRatio: 1
    readonly property var box: document.textBox
    property bool synchronizing: false
    signal contextRequested(var editor, real px, real py)
    visible: document.editingText
    width: box.width || 1
    height: box.height || 1

    function synchronize() {
        synchronizing = true;
        if (input.text !== (box.text || ""))
            input.text = box.text || "";
        synchronizing = false;
    }
    onVisibleChanged: {
        if (visible)
            Qt.callLater(function () {
                    root.synchronize();
                    input.forceActiveFocus();
                });
    }
    Connections {
        target: root.document
        function onTextBoxChanged() {
            if (root.visible)
                root.synchronize();
        }
    }
    transform: Matrix4x4 {
        matrix: Qt.matrix4x4((root.box.m11 === undefined ? 1 : root.box.m11) / root.pixelRatio, (root.box.m21 || 0) / root.pixelRatio, 0, ((root.box.dx || 0) + (root.box.m11 === undefined ? 1 : root.box.m11) * (root.box.x || 0) + (root.box.m21 || 0) * (root.box.y || 0)) / root.pixelRatio, (root.box.m12 || 0) / root.pixelRatio, (root.box.m22 === undefined ? 1 : root.box.m22) / root.pixelRatio, 0, ((root.box.dy || 0) + (root.box.m12 || 0) * (root.box.x || 0) + (root.box.m22 === undefined ? 1 : root.box.m22) * (root.box.y || 0)) / root.pixelRatio, 0, 0, 1, 0, 0, 0, 0, 1)
    }
    TextEdit {
        id: input
        x: 6
        y: 6
        width: Math.max(1, (root.box.width || 1) - 12)
        height: Math.max(1, (root.box.height || 1) - 12)
        clip: true
        color: root.box.color || "white"
        font.pixelSize: root.box.fontSize || 28
        textFormat: TextEdit.PlainText
        wrapMode: TextEdit.Wrap
        selectByMouse: true
        persistentSelection: true
        selectionColor: "#666666"
        selectedTextColor: "white"
        onTextChanged: {
            if (root.visible && !root.synchronizing)
                root.document.updateText(text);
        }
        onContentHeightChanged: Qt.callLater(function () {
                if (root.visible)
                    root.document.setTextContentHeight(input.contentHeight + 12);
            })
        Keys.onPressed: function (event) {
            if ((event.key === Qt.Key_Return || event.key === Qt.Key_Enter) && (event.modifiers & Qt.ControlModifier)) {
                root.document.finishTextEditing();
                event.accepted = true;
            }
        }
        MouseArea {
            anchors.fill: parent
            acceptedButtons: Qt.RightButton
            onPressed: function (mouse) {
                if (input.selectedText.length === 0)
                    input.cursorPosition = input.positionAt(mouse.x, mouse.y);
                const point = mapToItem(root.parent, mouse.x, mouse.y);
                root.contextRequested(input, point.x, point.y);
            }
        }
    }
}
