pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Dialogs
import SquigitAnnotation 1.0
import "../components"

Item {
    id: root
    required property var controller
    property bool ownsInput: false
    property bool composerFocused: false
    property real bottomInset: 24
    readonly property bool nativeDialogOpen: fileDialog.visible
    readonly property var document: controller.annotations
    readonly property bool textInput: document.editingText || composerFocused || colorPicker.visible
    readonly property bool shortcutsActive: visible && ownsInput && !fileDialog.visible && !toolMenu.visible && !objectMenu.visible && !textMenu.visible
    readonly property rect dockArea: Qt.rect(dock.x, dock.y, dock.width, dock.height)
    property real contextX: 0
    property real contextY: 0

    function closePopups() {
        if (colorPicker.visible) {
            colorPicker.close();
            return true;
        }
        if (toolMenu.visible) {
            toolMenu.close();
            return true;
        }
        if (objectMenu.visible) {
            objectMenu.close();
            return true;
        }
        if (textMenu.visible) {
            textMenu.close();
            return true;
        }
        return false;
    }
    function handleEscape() {
        return closePopups() || document.handleEscape();
    }
    function clearSelection() {
        closePopups();
        document.deselect();
    }
    function choose(tool) {
        document.selectTool(tool);
        surface.forceActiveFocus();
    }
    function formatAt(px, py) {
        const nearDock = py >= dock.y - 10 && py <= dock.y + dock.height;
        colorPicker.showAt(px, nearDock ? dock.y - colorPicker.height - 10 : py, document.selectionCount > 1);
    }
    function showOptions(tool, px, py) {
        contextX = px;
        contextY = py;
        if (tool === "eraser") {
            toolMenu.showAt(px, py, [{
                        "label": "Brush eraser",
                        "action": "eraser"
                    }, {
                        "label": "Object eraser",
                        "action": "objectEraser"
                    }, {
                        "separator": true
                    }, {
                        "label": "Clear all",
                        "action": "clear"
                    }]);
        } else if (tool === "shapes") {
            toolMenu.showAt(px, py, [{
                        "label": "Rectangle",
                        "action": "rectangle"
                    }, {
                        "label": "Arrow",
                        "action": "arrow"
                    }, {
                        "label": "Line",
                        "action": "line"
                    }, {
                        "label": "Circle",
                        "action": "ellipse"
                    }, {
                        "separator": true
                    }, {
                        "label": "Solid rectangle",
                        "action": "fillRectangle"
                    }, {
                        "label": "Solid circle",
                        "action": "fillEllipse"
                    }]);
        } else if (tool === "highlighter") {
            toolMenu.showAt(px, py, [{
                        "label": "Highlighter brush",
                        "action": "highlighter"
                    }, {
                        "label": "Solid rectangle highlighter",
                        "action": "highlightRectangle"
                    }]);
        } else if (tool === "copy") {
            toolMenu.showAt(px, py, [{
                        "label": "Copy annotated image",
                        "action": "copyImage"
                    }, {
                        "label": "Copy normal image",
                        "action": "copyNormal"
                    }]);
        } else if (tool === "export") {
            toolMenu.showAt(px, py, [{
                        "label": "Export annotated image",
                        "action": "exportImage"
                    }, {
                        "label": "Export normal image",
                        "action": "exportNormal"
                    }]);
        } else if (tool === "history") {
            toolMenu.showAt(px, py, [{
                        "label": "Undo",
                        "shortcut": "Ctrl Z",
                        "action": "undo",
                        "enabled": document.canUndo
                    }, {
                        "label": "Redo",
                        "shortcut": "Ctrl Shift Z",
                        "action": "redo",
                        "enabled": document.canRedo
                    }]);
        } else if (tool === "select") {
            toolMenu.showAt(px, py, [{
                        "label": "Select all",
                        "shortcut": "Ctrl A",
                        "action": "selectAll"
                    }, {
                        "label": "Format",
                        "action": "format",
                        "enabled": document.hasSelection
                    }]);
        } else {
            if (document.tool !== tool)
                choose(tool);
            formatAt(px, py);
        }
    }
    function run(action) {
        if (action === "focus")
            surface.forceActiveFocus();
        else if (action === "copyImage")
            controller.copyImage(true);
        else if (action === "copyNormal")
            controller.copyImage(false);
        else if (action === "exportImage")
            controller.exportImage(true);
        else if (action === "exportNormal")
            controller.exportImage(false);
        else if (action === "undo")
            document.undo();
        else if (action === "redo")
            document.redo();
        else if (action === "clear")
            document.clear();
        else if (action === "selectAll")
            document.selectAll();
        else if (action === "delete")
            document.deleteSelected();
        else if (action === "copy")
            document.copySelected();
        else if (action === "cut")
            document.cutSelected();
        else if (action === "paste")
            document.paste();
        else if (action === "insert")
            fileDialog.open();
        else if (action === "format")
            formatAt(contextX, contextY);
        else {
            if (["rectangle", "arrow", "line", "ellipse", "fillRectangle", "fillEllipse"].indexOf(action) >= 0)
                dock.shapeTool = action;
            if (action === "eraser" || action === "objectEraser")
                dock.eraseTool = action;
            if (action === "highlighter" || action === "highlightRectangle")
                dock.highlightTool = action;
            choose(action);
        }
    }

    AnnotationCanvas {
        id: surface
        anchors.fill: parent
        document: root.document
        pixelRatio: root.controller.pixelRatio
        onContextRequested: function (px, py, kind) {
            root.contextX = px;
            root.contextY = py;
            if (kind === "format")
                root.formatAt(px, py);
            else if (kind === "object")
                objectMenu.showAt(px, py, [{
                            "label": "Delete",
                            "shortcut": "Backspace",
                            "action": "delete"
                        }, {
                            "label": "Copy",
                            "shortcut": "Ctrl C",
                            "action": "copy"
                        }, {
                            "label": "Cut",
                            "shortcut": "Ctrl X",
                            "action": "cut"
                        }, {
                            "separator": true
                        }, {
                            "label": "Format",
                            "action": "format"
                        }]);
            else
                objectMenu.showAt(px, py, [{
                            "label": "Paste",
                            "shortcut": "Ctrl V",
                            "action": "paste"
                        }, {
                            "label": "Insert…",
                            "action": "insert"
                        }, {
                            "separator": true
                        }, {
                            "label": "Select all",
                            "shortcut": "Ctrl A",
                            "action": "selectAll"
                        }]);
        }
    }

    SelectionCrop {
        z: 1
        controller: root.controller
        annotationMode: true
        onCropInteractionStarted: root.clearSelection()
    }

    AnnotationTextEditor {
        z: 2
        document: root.document
        pixelRatio: root.controller.pixelRatio
        onContextRequested: function (editor, px, py) {
            textMenu.showFor(editor, px, py, true);
        }
    }

    AnnotationDock {
        id: dock
        z: 20
        document: root.document
        x: (root.width - width) / 2
        y: Math.max(12, root.height - height - root.bottomInset)
        onActionRequested: function (action) {
            root.run(action);
        }
        onOptionsRequested: function (tool, px, py) {
            root.showOptions(tool, px, py);
        }
    }

    Rectangle {
        id: notice
        z: 25
        x: Math.max(12, Math.min((root.width - width) / 2, root.width - width - 12))
        y: Math.max(12, dock.y - height - 12)
        width: Math.min(root.width - 24, noticeText.implicitWidth + 24)
        height: noticeText.implicitHeight + 16
        radius: 7
        color: "#292929"
        border.color: "#484848"
        visible: noticeTimer.running
        Text {
            id: noticeText
            anchors.fill: parent
            anchors.margins: 8
            text: root.document.status
            color: "#f5f5f5"
            font.pixelSize: 12
            horizontalAlignment: Text.AlignHCenter
            elide: Text.ElideMiddle
        }
    }
    Timer {
        id: noticeTimer
        interval: 4000
    }
    Connections {
        target: root.document
        function onStatusChanged() {
            noticeTimer.restart();
        }
    }
    AnnotationColorPicker {
        id: colorPicker
        parent: root
        document: root.document
    }
    AnnotationMenu {
        id: toolMenu
        parent: root
        aboveAnchor: true
        onTriggered: function (action) {
            root.run(action);
        }
    }
    AnnotationMenu {
        id: objectMenu
        parent: root
        onTriggered: function (action) {
            root.run(action);
        }
    }
    TextContextMenu {
        id: textMenu
        parent: root
        onFormatRequested: function (px, py) {
            root.formatAt(px, py);
        }
    }
    FileDialog {
        id: fileDialog
        title: "Insert image"
        fileMode: FileDialog.OpenFile
        nameFilters: ["Images (*.png *.jpg *.jpeg *.webp *.bmp *.gif *.svg)"]
        onVisibleChanged: surface.setDialogOpen(visible)
        onAccepted: {
            root.document.insertImage(selectedFile);
            surface.forceActiveFocus();
        }
        onRejected: surface.forceActiveFocus()
    }

    Repeater {
        model: [{
                "key": "V",
                "tool": "select"
            }, {
                "key": "B",
                "tool": "brush"
            }, {
                "key": "E",
                "tool": "eraser"
            }, {
                "key": "R",
                "tool": "rectangle"
            }, {
                "key": "C",
                "tool": "counter"
            }, {
                "key": "T",
                "tool": "text"
            }, {
                "key": "X",
                "tool": "blur"
            }, {
                "key": "H",
                "tool": "highlighter"
            }]
        delegate: Item {
            id: toolBinding
            required property var modelData
            Shortcut {
                sequence: toolBinding.modelData.key
                context: Qt.ApplicationShortcut
                enabled: root.shortcutsActive && !root.textInput
                onActivated: root.choose(toolBinding.modelData.tool === "rectangle" ? dock.shapeTool : toolBinding.modelData.tool === "eraser" ? dock.eraseTool : toolBinding.modelData.tool === "highlighter" ? dock.highlightTool : toolBinding.modelData.tool)
            }
        }
    }
    Repeater {
        model: [{
                "key": "Ctrl+Shift+C",
                "action": "copyImage"
            }, {
                "key": "Ctrl+S",
                "action": "exportImage"
            }, {
                "key": "Ctrl+Z",
                "action": "undo"
            }, {
                "key": "Ctrl+Shift+Z",
                "action": "redo"
            }, {
                "key": "Backspace",
                "action": "delete"
            }, {
                "key": "Delete",
                "action": "delete"
            }, {
                "key": "Ctrl+C",
                "action": "copy"
            }, {
                "key": "Ctrl+X",
                "action": "cut"
            }, {
                "key": "Ctrl+V",
                "action": "paste"
            }, {
                "key": "Ctrl+A",
                "action": "selectAll"
            }]
        delegate: Item {
            id: commandBinding
            required property var modelData
            Shortcut {
                sequence: commandBinding.modelData.key
                context: Qt.ApplicationShortcut
                enabled: root.shortcutsActive && (!root.textInput || commandBinding.modelData.action === "copyImage" || commandBinding.modelData.action === "exportImage")
                onActivated: root.run(commandBinding.modelData.action)
            }
        }
    }
}
