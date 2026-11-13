pragma ComponentBehavior: Bound

import QtQuick
import SquigitCapture 1.0

Item {
    id: root
    anchors.fill: parent

    required property var controller
    property bool annotationMode: false
    signal cropInteractionStarted
    readonly property rect area: controller.selection
    property bool interacting: false
    property rect startArea
    property point pressPoint
    property int horizontalEdge: 0
    property int verticalEdge: 0
    property bool moving: false
    readonly property string selectedText: ocrTextLayer.selectedText
    readonly property bool hasTextSelection: ocrTextLayer.hasTextSelection

    function clearTextSelection() {
        return ocrTextLayer.clearSelection();
    }

    function copyTextSelection() {
        ocrTextLayer.copySelection();
    }

    function selectAllText() {
        ocrTextLayer.selectAll();
    }

    function cropTargetAt(px, py) {
        for (let i = cropHandles.count - 1; i >= 0; --i) {
            const handle = cropHandles.itemAt(i);
            if (handle && px >= handle.x && py >= handle.y && px <= handle.x + handle.width && py <= handle.y + handle.height)
                return handle.modelData;
        }
        for (let i = cropBorders.count - 1; i >= 0; --i) {
            const border = cropBorders.itemAt(i);
            if (border && px >= border.x && py >= border.y && px <= border.x + border.width && py <= border.y + border.height)
                return {
                    "horizontal": 0,
                    "vertical": 0,
                    "cursor": Qt.SizeAllCursor
                };
        }
        return null;
    }

    function begin(item, mouse, horizontal, vertical, move) {
        clearTextSelection();
        cropInteractionStarted();
        startArea = area;
        pressPoint = item.mapToItem(root, mouse.x, mouse.y);
        horizontalEdge = horizontal;
        verticalEdge = vertical;
        moving = move;
        interacting = true;
    }

    function dragTo(item, mouse) {
        if (!interacting)
            return;
        const point = item.mapToItem(root, mouse.x, mouse.y);
        const dx = point.x - pressPoint.x;
        const dy = point.y - pressPoint.y;
        let left = startArea.x;
        let top = startArea.y;
        let right = left + startArea.width;
        let bottom = top + startArea.height;
        if (moving) {
            left = Math.max(0, Math.min(width - startArea.width, left + dx));
            top = Math.max(0, Math.min(height - startArea.height, top + dy));
            right = left + startArea.width;
            bottom = top + startArea.height;
        } else {
            const minimumWidth = Math.min(24, startArea.width);
            const minimumHeight = Math.min(24, startArea.height);
            if (horizontalEdge < 0)
                left = Math.max(0, Math.min(right - minimumWidth, left + dx));
            if (horizontalEdge > 0)
                right = Math.min(width, Math.max(left + minimumWidth, right + dx));
            if (verticalEdge < 0)
                top = Math.max(0, Math.min(bottom - minimumHeight, top + dy));
            if (verticalEdge > 0)
                bottom = Math.min(height, Math.max(top + minimumHeight, bottom + dy));
        }
        controller.setSelection(Qt.rect(left, top, right - left, bottom - top));
    }

    Rectangle {
        x: 0
        y: 0
        width: root.width
        height: root.area.y
        color: "#70000000"
    }
    Rectangle {
        x: 0
        y: root.area.y + root.area.height
        width: root.width
        height: root.height - y
        color: "#70000000"
    }
    Rectangle {
        x: 0
        y: root.area.y
        width: root.area.x
        height: root.area.height
        color: "#70000000"
    }
    Rectangle {
        x: root.area.x + root.area.width
        y: root.area.y
        width: root.width - x
        height: root.area.height
        color: "#70000000"
    }

    OcrTextLayer {
        id: ocrTextLayer
        controller: root.controller
        interacting: root.interacting
        visible: !root.annotationMode && root.controller.ocrVisible
    }

    Rectangle {
        x: root.area.x
        y: root.area.y
        width: root.area.width
        height: root.area.height
        color: "transparent"
        border.color: "white"
        border.width: 1
    }

    Repeater {
        id: cropBorders
        model: 4
        delegate: MouseArea {
            id: borderMove
            required property int index
            enabled: root.annotationMode
            x: index === 3 ? root.area.x + root.area.width - 3 : root.area.x - 3
            y: index === 2 ? root.area.y + root.area.height - 3 : root.area.y - 3
            width: index % 2 === 0 ? root.area.width + 6 : 6
            height: index % 2 === 0 ? 6 : root.area.height + 6
            preventStealing: true
            hoverEnabled: true
            cursorShape: Qt.SizeAllCursor
            onPressed: function (mouse) {
                root.begin(borderMove, mouse, 0, 0, true);
            }
            onPositionChanged: function (mouse) {
                if (pressed)
                    root.dragTo(borderMove, mouse);
            }
            onReleased: function (mouse) {
                root.dragTo(borderMove, mouse);
                root.interacting = false;
            }
            onCanceled: root.interacting = false
        }
    }

    Repeater {
        id: cropHandles
        model: [{
                "horizontal": -1,
                "vertical": -1,
                "cursor": Qt.SizeFDiagCursor
            }, {
                "horizontal": 1,
                "vertical": -1,
                "cursor": Qt.SizeBDiagCursor
            }, {
                "horizontal": -1,
                "vertical": 1,
                "cursor": Qt.SizeBDiagCursor
            }, {
                "horizontal": 1,
                "vertical": 1,
                "cursor": Qt.SizeFDiagCursor
            }, {
                "horizontal": -1,
                "vertical": 0,
                "cursor": Qt.SizeHorCursor
            }, {
                "horizontal": 1,
                "vertical": 0,
                "cursor": Qt.SizeHorCursor
            }, {
                "horizontal": 0,
                "vertical": -1,
                "cursor": Qt.SizeVerCursor
            }, {
                "horizontal": 0,
                "vertical": 1,
                "cursor": Qt.SizeVerCursor
            }]

        delegate: Item {
            id: handle
            required property var modelData
            x: root.area.x + root.area.width * (modelData.horizontal + 1) / 2 - width / 2
            y: root.area.y + root.area.height * (modelData.vertical + 1) / 2 - height / 2
            width: 24
            height: 24

            Rectangle {
                anchors.centerIn: parent
                width: 8
                height: 8
                radius: 2
                color: "white"
                border.color: "#404040"
            }

            MouseArea {
                id: resizeMouse
                anchors.fill: parent
                enabled: root.annotationMode
                preventStealing: true
                hoverEnabled: true
                cursorShape: handle.modelData.cursor
                onPressed: function (mouse) {
                    root.begin(resizeMouse, mouse, handle.modelData.horizontal, handle.modelData.vertical, false);
                }
                onPositionChanged: function (mouse) {
                    if (pressed)
                        root.dragTo(resizeMouse, mouse);
                }
                onReleased: function (mouse) {
                    root.dragTo(resizeMouse, mouse);
                    root.interacting = false;
                }
                onCanceled: root.interacting = false
            }
        }
    }

    CaptureCursor {
        id: cursorSurface
        anchors.fill: parent
        enabled: !root.annotationMode
        onHoverMoved: function (px, py) {
            if (root.interacting)
                cursorSurface.cursorShape = pointer.pressedCursor;
            else if (pointer.selectingText)
                cursorSurface.cursorShape = Qt.IBeamCursor;
            else {
                const target = root.cropTargetAt(px, py);
                cursorSurface.cursorShape = target ? target.cursor : ocrTextLayer.lineAt(px, py) >= 0 ? Qt.IBeamCursor : Qt.ArrowCursor;
            }
        }

        Connections {
            target: root.controller
            function onOcrChanged() {
                Qt.callLater(function () {
                        cursorSurface.refresh();
                    });
            }
            function onSelectionChanged() {
                cursorSurface.refresh();
            }
        }

        MouseArea {
            id: pointer
            anchors.fill: parent
            acceptedButtons: Qt.LeftButton
            preventStealing: true
            property bool selectingText: false
            property int pressedCursor: Qt.ArrowCursor

            onPressed: function (mouse) {
                const target = root.cropTargetAt(mouse.x, mouse.y);
                selectingText = false;
                if (target) {
                    pressedCursor = target.cursor;
                    root.begin(pointer, mouse, target.horizontal, target.vertical, target.horizontal === 0 && target.vertical === 0);
                } else {
                    const line = ocrTextLayer.lineAt(mouse.x, mouse.y);
                    if (line >= 0) {
                        selectingText = true;
                        ocrTextLayer.beginTextSelection(line, mouse.x, mouse.y, mouse.modifiers);
                    } else {
                        root.clearTextSelection();
                    }
                }
                cursorSurface.hoverAt(Qt.point(mouse.x, mouse.y));
            }
            onPositionChanged: function (mouse) {
                if (!pressed)
                    return;
                if (root.interacting)
                    root.dragTo(pointer, mouse);
                else if (selectingText)
                    ocrTextLayer.dragSelection(mouse.x, mouse.y);
                cursorSurface.hoverAt(Qt.point(mouse.x, mouse.y));
            }
            onReleased: function (mouse) {
                if (root.interacting) {
                    root.dragTo(pointer, mouse);
                    root.interacting = false;
                } else if (selectingText) {
                    ocrTextLayer.dragSelection(mouse.x, mouse.y);
                }
                selectingText = false;
                cursorSurface.hoverAt(Qt.point(mouse.x, mouse.y));
            }
            onDoubleClicked: function (mouse) {
                if (root.interacting || root.cropTargetAt(mouse.x, mouse.y))
                    return;
                const line = ocrTextLayer.lineAt(mouse.x, mouse.y);
                if (line >= 0) {
                    selectingText = true;
                    ocrTextLayer.selectWordAt(line, mouse.x, mouse.y);
                }
                cursorSurface.hoverAt(Qt.point(mouse.x, mouse.y));
            }
            onCanceled: {
                root.interacting = false;
                if (selectingText)
                    root.clearTextSelection();
                selectingText = false;
                cursorSurface.refresh();
            }
        }
    }
}
