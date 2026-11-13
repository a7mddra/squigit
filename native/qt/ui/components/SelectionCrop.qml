pragma ComponentBehavior: Bound

import QtQuick
import Qt5Compat.GraphicalEffects
import "../annotation"

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

    function expandToDisplay() {
        clearTextSelection();
        cropInteractionStarted();
        controller.setSelection(Qt.rect(0, 0, width, height));
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

    MouseArea {
        anchors.fill: parent
        visible: !root.annotationMode
        acceptedButtons: Qt.LeftButton
        hoverEnabled: true
        preventStealing: true
        cursorShape: Qt.ArrowCursor
        onPressed: root.clearTextSelection()
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

    Item {
        id: fullScreenButton
        readonly property bool fitsOutside: root.area.x + root.area.width + width + 24 <= root.width
        x: Math.max(8, Math.min(root.width - width - 8, fitsOutside ? root.area.x + root.area.width + 16 : root.area.x + root.area.width - width - 16))
        y: Math.max(8, Math.min(root.height - height - 8, fitsOutside ? root.area.y - height / 2 : root.area.y + 16))
        width: 32
        height: 32

        Image {
            anchors.centerIn: parent
            width: 22
            height: 22
            source: "icons/full-screen.svg"
            sourceSize: Qt.size(44, 44)
            opacity: fullScreenMouse.pressed ? 0.6 : fullScreenMouse.containsMouse ? 1 : 0.85
            layer.enabled: true
            layer.effect: DropShadow {
                horizontalOffset: 0
                verticalOffset: 1
                radius: 3
                samples: 7
                color: "#b0000000"
            }
        }

        MouseArea {
            id: fullScreenMouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: root.expandToDisplay()
        }

        AnnotationToolTip {
            text: "Full screen"
            visible: fullScreenMouse.containsMouse && !fullScreenMouse.pressed && !root.interacting
            x: Math.max(8 - fullScreenButton.x, Math.min((fullScreenButton.width - width) / 2, root.width - fullScreenButton.x - width - 8))
            y: fullScreenButton.y < height + 8 ? fullScreenButton.height + 8 : -height - 8
        }
    }
}
