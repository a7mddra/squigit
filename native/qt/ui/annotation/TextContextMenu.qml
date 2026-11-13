pragma ComponentBehavior: Bound

import QtQuick

AnnotationMenu {
    id: root
    property var editor
    property bool allowFormat: false
    signal formatRequested(real px, real py)

    function showFor(target, px, py, format) {
        editor = target;
        allowFormat = format;
        const items = [{
                "label": "Undo",
                "shortcut": "Ctrl Z",
                "action": "undo",
                "enabled": target.canUndo
            }, {
                "label": "Redo",
                "shortcut": "Ctrl Shift Z",
                "action": "redo",
                "enabled": target.canRedo
            }, {
                "separator": true
            }, {
                "label": "Cut",
                "shortcut": "Ctrl X",
                "action": "cut",
                "enabled": target.selectedText.length > 0
            }, {
                "label": "Copy",
                "shortcut": "Ctrl C",
                "action": "copy",
                "enabled": target.selectedText.length > 0
            }, {
                "label": "Paste",
                "shortcut": "Ctrl V",
                "action": "paste",
                "enabled": target.canPaste
            }, {
                "separator": true
            }, {
                "label": "Select all",
                "shortcut": "Ctrl A",
                "action": "selectAll",
                "enabled": target.length > 0
            }];
        if (format) {
            items.push({
                    "separator": true
                });
            items.push({
                    "label": "Format",
                    "action": "format"
                });
        }
        showAt(px, py, items);
    }

    onTriggered: function (action) {
        if (!editor)
            return;
        if (action === "format") {
            root.formatRequested(x, y);
            return;
        }
        editor.forceActiveFocus();
        if (action === "undo")
            editor.undo();
        else if (action === "redo")
            editor.redo();
        else if (action === "cut")
            editor.cut();
        else if (action === "copy")
            editor.copy();
        else if (action === "paste")
            editor.paste();
        else if (action === "selectAll")
            editor.selectAll();
    }
}
