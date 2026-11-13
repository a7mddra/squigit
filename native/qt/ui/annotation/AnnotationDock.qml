pragma ComponentBehavior: Bound

import QtQuick
import Qt5Compat.GraphicalEffects

Item {
    id: root
    required property var document
    property string shapeTool: "rectangle"
    property string eraseTool: "eraser"
    property string highlightTool: "highlighter"
    property bool menusOpen: false
    property var tipButton: null
    property var suppressedTipButton: null
    readonly property real popupOverlap: 4
    signal actionRequested(string action)
    signal optionsRequested(string tool, real px, real py)
    width: Math.min(tools.width + 16, parent.width - 24)
    height: 54

    function showTip(button) {
        if (suppressedTipButton === button)
            return;
        tipButton = button;
        toolTip.visible = true;
    }
    function leaveTip(button) {
        if (suppressedTipButton === button)
            suppressedTipButton = null;
        if (tipButton === button) {
            toolTip.visible = false;
            tipButton = null;
        }
    }
    function dismissTips() {
        if (tipButton)
            suppressedTipButton = tipButton;
        toolTip.visible = false;
        tipButton = null;
    }
    function choose(tool) {
        document.selectTool(tool);
        actionRequested("focus");
    }
    function options(button, tool) {
        const point = button.mapToItem(root.parent, button.width, 0);
        optionsRequested(tool, point.x, root.y + popupOverlap);
    }
    function confirmCopy() {
        dismissTips();
        copyButton.showConfirmation();
    }
    function confirmExport() {
        dismissTips();
        exportButton.showConfirmation();
    }
    AnnotationToolTip {
        id: toolTip
        parent: root
        text: root.tipButton ? root.tipButton.tip : ""
        delay: root.menusOpen ? 1000 : 550
        x: {
            const center = root.tipButton ? root.tipButton.mapToItem(root, root.tipButton.width / 2, 0).x : root.width / 2;
            return Math.max(8 - root.x, Math.min(center - width / 2, root.parent.width - root.x - width - 8));
        }
        y: root.popupOverlap - height
    }

    Rectangle {
        anchors.fill: parent
        color: "#242424"
        radius: 14
        border.color: "#494949"
        layer.enabled: true
        layer.effect: DropShadow {
            horizontalOffset: 0
            verticalOffset: 6
            radius: 24
            samples: 49
            color: "#90000000"
        }
    }
    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.AllButtons
    }
    Flickable {
        anchors.fill: parent
        anchors.margins: 8
        contentWidth: tools.width
        contentHeight: height
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        flickableDirection: Flickable.HorizontalFlick

        Row {
            id: tools
            spacing: 4
            AnnotationToolButton {
                id: selectButton
                tooltipHost: root
                iconName: "select"
                tip: "Select · V"
                selected: root.document.tool === "select"
                onClicked: root.choose("select")
            }
            AnnotationToolButton {
                id: brushButton
                alternateEnabled: true
                tooltipHost: root
                iconName: "brush"
                tip: "Brush · B"
                selected: root.document.tool === "brush"
                onClicked: root.choose("brush")
                onAlternateRequested: root.options(brushButton, "brush")
            }
            AnnotationToolButton {
                id: eraseButton
                alternateEnabled: true
                tooltipHost: root
                chevron: true
                iconName: "eraser"
                tip: root.eraseTool === "eraser" ? "Brush eraser · E" : "Object eraser · E"
                selected: root.document.tool === "eraser" || root.document.tool === "objectEraser"
                onClicked: root.choose(root.eraseTool)
                onAlternateRequested: root.options(eraseButton, "eraser")
            }
            AnnotationToolButton {
                id: shapeButton
                alternateEnabled: true
                tooltipHost: root
                chevron: true
                iconName: root.shapeTool
                tip: "Shapes · R"
                selected: ["rectangle", "arrow", "line", "ellipse", "fillRectangle", "fillEllipse"].indexOf(root.document.tool) >= 0
                onClicked: root.choose(root.shapeTool)
                onAlternateRequested: root.options(shapeButton, "shapes")
            }
            AnnotationToolButton {
                id: counterButton
                alternateEnabled: true
                tooltipHost: root
                numberLabel: String(root.document.nextCounter)
                tip: "Counter · C"
                selected: root.document.tool === "counter"
                onClicked: root.choose("counter")
                onAlternateRequested: root.options(counterButton, "counter")
            }
            AnnotationToolButton {
                id: textButton
                alternateEnabled: true
                tooltipHost: root
                iconName: "text"
                tip: "Text · T"
                selected: root.document.tool === "text"
                onClicked: root.choose("text")
                onAlternateRequested: root.options(textButton, "text")
            }
            AnnotationToolButton {
                id: blurButton
                tooltipHost: root
                iconName: "blur"
                tip: "Blur · X"
                selected: root.document.tool === "blur"
                onClicked: root.choose("blur")
            }
            AnnotationToolButton {
                id: highlighterButton
                alternateEnabled: true
                tooltipHost: root
                chevron: true
                iconName: root.highlightTool === "highlighter" ? "highlighter" : "fillRectangle"
                tip: "Highlighter · H"
                selected: root.document.tool === "highlighter" || root.document.tool === "highlightRectangle"
                onClicked: root.choose(root.highlightTool)
                onAlternateRequested: root.options(highlighterButton, "highlighter")
            }
            Rectangle {
                width: 1
                height: 24
                anchors.verticalCenter: parent.verticalCenter
                color: "#505050"
            }
            AnnotationToolButton {
                id: copyButton
                alternateEnabled: true
                tooltipHost: root
                chevron: true
                iconName: "copy"
                tip: "Copy annotated image · Ctrl Shift C"
                onClicked: root.actionRequested("copyImage")
                onAlternateRequested: root.options(copyButton, "copy")
            }
            AnnotationToolButton {
                id: exportButton
                alternateEnabled: true
                tooltipHost: root
                chevron: true
                iconName: "export"
                tip: "Export annotated image · Ctrl S"
                onClicked: root.actionRequested("exportImage")
                onAlternateRequested: root.options(exportButton, "export")
            }
            Rectangle {
                width: 1
                height: 24
                anchors.verticalCenter: parent.verticalCenter
                color: "#505050"
            }
            AnnotationToolButton {
                id: undoButton
                tooltipHost: root
                iconName: "undo"
                tip: "Undo · Ctrl Z"
                enabled: root.document.canUndo
                onClicked: root.document.undo()
            }
            AnnotationToolButton {
                id: redoButton
                tooltipHost: root
                iconName: "redo"
                tip: "Redo · Ctrl Shift Z"
                enabled: root.document.canRedo
                onClicked: root.document.redo()
            }
        }
    }
}
