pragma ComponentBehavior: Bound

import QtQuick
import Qt5Compat.GraphicalEffects

Item {
    id: root
    required property var document
    property string shapeTool: "rectangle"
    property string eraseTool: "eraser"
    property string highlightTool: "highlighter"
    signal actionRequested(string action)
    signal optionsRequested(string tool, real px, real py)
    width: Math.min(tools.width + 16, parent.width - 24)
    height: 54

    function choose(tool) {
        document.selectTool(tool);
        actionRequested("focus");
    }
    function options(button, tool) {
        const point = button.mapToItem(root.parent, button.width, -8);
        optionsRequested(tool, point.x, point.y);
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
                iconName: "select"
                tip: "Select · V"
                selected: root.document.tool === "select"
                onClicked: root.choose("select")
                onAlternateRequested: root.options(selectButton, "select")
            }
            AnnotationToolButton {
                id: brushButton
                iconName: "brush"
                tip: "Brush · B"
                selected: root.document.tool === "brush"
                onClicked: root.choose("brush")
                onAlternateRequested: root.options(brushButton, "brush")
            }
            AnnotationToolButton {
                id: eraseButton
                iconName: "eraser"
                tip: root.eraseTool === "eraser" ? "Brush eraser · E" : "Object eraser · E"
                selected: root.document.tool === "eraser" || root.document.tool === "objectEraser"
                onClicked: root.choose(root.eraseTool)
                onAlternateRequested: root.options(eraseButton, "eraser")
            }
            AnnotationToolButton {
                id: shapeButton
                iconName: root.shapeTool
                tip: "Shapes · R"
                selected: ["rectangle", "arrow", "line", "ellipse", "fillRectangle", "fillEllipse"].indexOf(root.document.tool) >= 0
                onClicked: root.choose(root.shapeTool)
                onAlternateRequested: root.options(shapeButton, "shapes")
            }
            AnnotationToolButton {
                id: counterButton
                iconName: "counter"
                tip: "Counter · C"
                selected: root.document.tool === "counter"
                onClicked: root.choose("counter")
                onAlternateRequested: root.options(counterButton, "counter")
            }
            AnnotationToolButton {
                id: textButton
                iconName: "text"
                tip: "Text · T"
                selected: root.document.tool === "text"
                onClicked: root.choose("text")
                onAlternateRequested: root.options(textButton, "text")
            }
            AnnotationToolButton {
                id: blurButton
                iconName: "blur"
                tip: "Blur · X"
                selected: root.document.tool === "blur"
                onClicked: root.choose("blur")
                onAlternateRequested: root.options(blurButton, "blur")
            }
            AnnotationToolButton {
                id: highlighterButton
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
                iconName: "copy"
                tip: "Copy annotated image · Ctrl Shift C"
                onClicked: root.actionRequested("copyImage")
                onAlternateRequested: root.options(copyButton, "copy")
            }
            AnnotationToolButton {
                id: exportButton
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
                iconName: "undo"
                tip: "Undo · Ctrl Z"
                enabled: root.document.canUndo
                onClicked: root.document.undo()
                onAlternateRequested: root.options(undoButton, "history")
            }
            AnnotationToolButton {
                id: redoButton
                iconName: "redo"
                tip: "Redo · Ctrl Shift Z"
                enabled: root.document.canRedo
                onClicked: root.document.redo()
                onAlternateRequested: root.options(redoButton, "history")
            }
        }
    }
}
