pragma ComponentBehavior: Bound

import QtQuick
import SquigitCapture 1.0

Item {
    id: root
    anchors.fill: parent

    required property var controller
    property bool annotationMode: false
    signal cropInteractionStarted
    signal contextRequested(real px, real py)
    readonly property rect area: visible ? controller.selection : Qt.rect(0, 0, 0, 0)
    property bool interacting: false
    property rect startArea
    property point pressPoint
    property point pendingPoint
    property bool dragPending: false
    property int horizontalEdge: 0
    property int verticalEdge: 0
    property bool moving: false
    property int dragCursorShape: Qt.ArrowCursor
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

    function expandToDisplay() {
        cancelInteraction();
        clearTextSelection();
        cropInteractionStarted();
        controller.setSelection(Qt.rect(0, 0, width, height));
    }

    function begin(item, mouse, horizontal, vertical, move, cursor) {
        cancelInteraction();
        startArea = area;
        pressPoint = item.mapToItem(root, mouse.x, mouse.y);
        horizontalEdge = horizontal;
        verticalEdge = vertical;
        moving = move;
        dragCursorShape = cursor;
        controller.beginCropInteraction();
        interacting = true;
        clearTextSelection();
        cropInteractionStarted();
    }

    function dragTo(item, mouse) {
        if (!interacting)
            return;
        pendingPoint = item.mapToItem(root, mouse.x, mouse.y);
        dragPending = true;
        if (!dragFrame.running)
            dragFrame.start();
    }

    function flushDrag() {
        if (!interacting || !dragPending)
            return;
        dragPending = false;
        const dx = pendingPoint.x - pressPoint.x;
        const dy = pendingPoint.y - pressPoint.y;
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

    function finish(item, mouse) {
        if (!interacting)
            return;
        dragTo(item, mouse);
        dragFrame.stop();
        flushDrag();
        controller.endCropInteraction();
        interacting = false;
    }

    function cancelInteraction() {
        if (!interacting)
            return;
        dragFrame.stop();
        dragPending = false;
        controller.endCropInteraction();
        interacting = false;
    }

    onVisibleChanged: {
        if (!visible)
            cancelInteraction();
    }
    onEnabledChanged: {
        if (!enabled)
            cancelInteraction();
    }

    Timer {
        id: dragFrame
        interval: 16
        onTriggered: root.flushDrag()
    }

    CropOverlay {
        anchors.fill: parent
        selection: root.area
    }

    MouseArea {
        anchors.fill: parent
        visible: !root.annotationMode
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        hoverEnabled: true
        preventStealing: true
        cursorShape: root.interacting ? root.dragCursorShape : Qt.ArrowCursor
        onPressed: function (mouse) {
            if (mouse.button === Qt.RightButton)
                root.contextRequested(mouse.x, mouse.y);
            else
                root.clearTextSelection();
        }
    }

    OcrTextLayer {
        id: ocrTextLayer
        controller: root.controller
        interacting: root.interacting
        autoSelectOnRecognition: !root.annotationMode && root.controller.action === "ocr"
        visible: !root.annotationMode && root.controller.ocrVisible
    }

    MouseArea {
        id: qrMove
        x: root.area.x
        y: root.area.y
        width: root.area.width
        height: root.area.height
        visible: !root.annotationMode && root.controller.action === "scan-qr"
        acceptedButtons: Qt.LeftButton
        hoverEnabled: true
        preventStealing: true
        cursorShape: root.interacting ? root.dragCursorShape : pressed ? Qt.ClosedHandCursor : Qt.OpenHandCursor
        onPressed: function (mouse) {
            root.begin(qrMove, mouse, 0, 0, true, Qt.ClosedHandCursor);
        }
        onPositionChanged: function (mouse) {
            if (pressed)
                root.dragTo(qrMove, mouse);
        }
        onReleased: function (mouse) {
            root.finish(qrMove, mouse);
        }
        onCanceled: root.cancelInteraction()
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
        model: 4
        delegate: MouseArea {
            id: borderMove
            required property int index
            x: index === 3 ? root.area.x + root.area.width - 3 : root.area.x - 3
            y: index === 2 ? root.area.y + root.area.height - 3 : root.area.y - 3
            width: index % 2 === 0 ? root.area.width + 6 : 6
            height: index % 2 === 0 ? 6 : root.area.height + 6
            preventStealing: true
            hoverEnabled: true
            cursorShape: root.interacting ? root.dragCursorShape : Qt.SizeAllCursor
            onPressed: function (mouse) {
                root.begin(borderMove, mouse, 0, 0, true, Qt.SizeAllCursor);
            }
            onPositionChanged: function (mouse) {
                if (pressed)
                    root.dragTo(borderMove, mouse);
            }
            onReleased: function (mouse) {
                root.finish(borderMove, mouse);
            }
            onCanceled: root.cancelInteraction()
        }
    }

    Repeater {
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
            x: Math.max(4, Math.min(root.width - 4, root.area.x + root.area.width * (modelData.horizontal + 1) / 2)) - width / 2
            y: Math.max(4, Math.min(root.height - 4, root.area.y + root.area.height * (modelData.vertical + 1) / 2)) - height / 2
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
                preventStealing: true
                hoverEnabled: true
                cursorShape: root.interacting ? root.dragCursorShape : handle.modelData.cursor
                onPressed: function (mouse) {
                    root.begin(resizeMouse, mouse, handle.modelData.horizontal, handle.modelData.vertical, false, handle.modelData.cursor);
                }
                onPositionChanged: function (mouse) {
                    if (pressed)
                        root.dragTo(resizeMouse, mouse);
                }
                onReleased: function (mouse) {
                    root.finish(resizeMouse, mouse);
                }
                onCanceled: root.cancelInteraction()
            }
        }
    }
}
