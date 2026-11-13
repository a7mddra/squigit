pragma ComponentBehavior: Bound

import QtQuick

FocusScope {
    id: root
    anchors.fill: parent

    required property var controller
    property bool interacting: false
    property string selectedText: ""
    property int anchorLine: -1
    property int anchorPosition: 0
    property int cursorLine: -1
    property int cursorPosition: 0
    property bool wordMode: false
    property int wordStart: 0
    property int wordEnd: 0
    readonly property bool hasTextSelection: selectedText.length > 0
    visible: controller.ocrVisible
    enabled: visible && !interacting
    focus: visible

    function clearSelection() {
        const hadSelection = hasTextSelection;
        anchorLine = -1;
        cursorLine = -1;
        wordMode = false;
        applySelection();
        if (visible && enabled)
            forceActiveFocus();
        return hadSelection;
    }

    function applySelection() {
        const forward = anchorLine < cursorLine || (anchorLine === cursorLine && anchorPosition <= cursorPosition);
        const first = forward ? anchorLine : cursorLine;
        const last = forward ? cursorLine : anchorLine;
        const start = forward ? anchorPosition : cursorPosition;
        const end = forward ? cursorPosition : anchorPosition;
        const parts = [];
        let previousRegion = -1;
        for (let i = 0; i < textLines.count; ++i) {
            const line = textLines.itemAt(i);
            if (!line)
                continue;
            const selected = first >= 0 && i >= first && i <= last && line.modelData.inside;
            const from = selected && i === first ? start : 0;
            const to = selected ? (i === last ? end : line.modelData.text.length) : 0;
            line.selectRange(from, to);
            if (selected) {
                const text = line.modelData.text.slice(from, to);
                if (parts.length > 0 && previousRegion === line.modelData.region)
                    parts[parts.length - 1] += text;
                else
                    parts.push(text);
                previousRegion = line.modelData.region;
            }
        }
        selectedText = parts.join("\n");
    }

    function nearestLine(px, py) {
        let found = -1;
        let verticalDistance = Infinity;
        let horizontalDistance = Infinity;
        for (let i = 0; i < textLines.count; ++i) {
            const line = textLines.itemAt(i);
            if (!line || !line.modelData.inside)
                continue;
            const point = line.mapFromItem(root, px, py);
            const dx = Math.max(0, -point.x, point.x - line.width);
            const dy = Math.max(0, -point.y, point.y - line.height);
            if (dx === 0 && dy === 0)
                return i;
            if (dy < verticalDistance || (dy === verticalDistance && dx < horizontalDistance)) {
                found = i;
                verticalDistance = dy;
                horizontalDistance = dx;
            }
        }
        return found;
    }

    function dragSelection(px, py) {
        const index = nearestLine(px, py);
        if (index < 0 || anchorLine < 0)
            return;
        const line = textLines.itemAt(index);
        const position = line.positionAt(px, py);
        cursorLine = index;
        if (wordMode) {
            const word = line.wordAt(position);
            const before = index < anchorLine || (index === anchorLine && position < wordStart);
            anchorPosition = before ? wordEnd : wordStart;
            cursorPosition = before ? word.start : word.end;
        } else {
            cursorPosition = position;
        }
        applySelection();
    }

    function beginTextSelection(index, px, py, modifiers) {
        const position = textLines.itemAt(index).positionAt(px, py);
        if (!(modifiers & Qt.ShiftModifier) || anchorLine < 0) {
            anchorLine = index;
            anchorPosition = position;
        }
        cursorLine = index;
        cursorPosition = position;
        wordMode = false;
        applySelection();
        forceActiveFocus();
    }

    function selectWordAt(index, px, py) {
        const line = textLines.itemAt(index);
        const word = line.wordAt(line.positionAt(px, py));
        anchorLine = index;
        cursorLine = index;
        wordStart = word.start;
        wordEnd = word.end;
        anchorPosition = word.start;
        cursorPosition = word.end;
        wordMode = true;
        applySelection();
    }

    function selectAll() {
        anchorLine = -1;
        cursorLine = -1;
        wordMode = false;
        for (let i = 0; i < textLines.count; ++i) {
            const line = textLines.itemAt(i);
            if (!line || !line.modelData.inside)
                continue;
            if (anchorLine < 0)
                anchorLine = i;
            cursorLine = i;
            cursorPosition = line.modelData.text.length;
        }
        anchorPosition = 0;
        applySelection();
        if (visible && enabled)
            forceActiveFocus();
    }

    function copySelection() {
        if (hasTextSelection)
            controller.copyTextSelection(selectedText);
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

    Connections {
        target: root.controller
        function onOcrChanged() {
            root.clearSelection();
        }
        function onSelectionChanged() {
            root.clearSelection();
        }
    }

    Repeater {
        id: textLines
        model: root.controller.ocrVisible ? root.controller.textBoxes : []

        delegate: Item {
            id: line
            required property var modelData
            required property int index
            readonly property var box: modelData.box
            readonly property real ratio: root.controller.pixelRatio

            function selectRange(start, end) {
                if (selectableLine.selectionStart !== start || selectableLine.selectionEnd !== end)
                    selectableLine.select(start, end);
            }

            function positionAt(px, py) {
                const point = selectableLine.mapFromItem(root, px, py);
                return selectableLine.positionAt(Math.max(0, Math.min(selectableLine.width, point.x)), selectableLine.height / 2);
            }

            function wordAt(position) {
                selectableLine.cursorPosition = position;
                selectableLine.selectWord();
                return {
                    "start": selectableLine.selectionStart,
                    "end": selectableLine.selectionEnd
                };
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
                enabled: false
                selectByMouse: false
                activeFocusOnPress: false
                persistentSelection: true
                color: "transparent"
                selectedTextColor: "transparent"
                selectionColor: "#805daaff"
                transform: Scale {
                    xScale: line.width / selectableLine.width
                }
            }
            MouseArea {
                anchors.fill: parent
                visible: line.modelData.inside
                acceptedButtons: Qt.LeftButton
                hoverEnabled: true
                preventStealing: true
                cursorShape: Qt.IBeamCursor
                onPressed: function (mouse) {
                    const point = mapToItem(root, mouse.x, mouse.y);
                    root.beginTextSelection(line.index, point.x, point.y, mouse.modifiers);
                }
                onPositionChanged: function (mouse) {
                    if (pressed) {
                        const point = mapToItem(root, mouse.x, mouse.y);
                        root.dragSelection(point.x, point.y);
                    }
                }
                onReleased: function (mouse) {
                    const point = mapToItem(root, mouse.x, mouse.y);
                    root.dragSelection(point.x, point.y);
                }
                onDoubleClicked: function (mouse) {
                    const point = mapToItem(root, mouse.x, mouse.y);
                    root.selectWordAt(line.index, point.x, point.y);
                }
                onCanceled: root.clearSelection()
            }
        }
    }
}
