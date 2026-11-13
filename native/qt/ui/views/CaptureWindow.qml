pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Window
import "../components"
import "../annotation"

Window {
    id: root

    flags: Qt.FramelessWindowHint | Qt.WindowStaysOnTopHint | Qt.Tool

    x: Screen.virtualX
    y: Screen.virtualY
    width: Screen.width
    height: Screen.height

    onClosing: function (close) {
        close.accepted = false;
        root.controller.cancel();
    }

    color: "transparent"

    required property var controller
    property bool ownsInput: false
    readonly property bool annotationMode: controller.action === "snapshot" || controller.action === "ask-google"

    function escapePressed() {
        exitHint.beginEscape();
    }

    function escapeTapped() {
        exitHint.replayHint();
        if (askComposer.dismissContextMenu())
            return;
        if (annotationMode && annotationEditor.handleEscape())
            return;
        selectionCrop.clearTextSelection();
    }

    function runOcrAction(action) {
        if (action === "selectAll")
            selectionCrop.selectAllText();
        else if (action === "copy")
            selectionCrop.copyTextSelection();
        else if (action === "search" || action === "translate")
            controller.openOcrText(selectionCrop.selectedText, action === "translate");
        else if (action === "visibility" || action === "retry")
            controller.toggleOcr();
    }

    Image {
        id: background
        anchors.fill: parent
        source: root.controller.backgroundSource
        fillMode: Image.Stretch
        smooth: false
        mipmap: false
        asynchronous: false
        cache: false

        onStatusChanged: {
            if (status === Image.Error) {
                const fallbackSource = root.controller.ensureBackgroundFileFallback();
                if (fallbackSource && source !== fallbackSource) {
                    source = fallbackSource;
                }
            }
        }
    }

    Rectangle {
        id: dimOverlay
        anchors.fill: parent
        visible: !root.controller.hasSelection && root.controller.captureMode !== "traditional"

        gradient: Gradient {
            GradientStop {
                position: 0.0
                color: Qt.rgba(0, 0, 0, 0.5)
            }
            GradientStop {
                position: 1.0
                color: "transparent"
            }
        }

        NumberAnimation on opacity  {
            from: 0
            to: 1
            duration: 200
            running: dimOverlay.visible
            easing.type: Easing.OutQuad
        }
    }

    Loader {
        id: canvasLoader
        anchors.fill: parent
        focus: true
        visible: !root.controller.hasSelection
        enabled: visible

        source: root.controller.captureMode === "traditional" ? "../components/TraditionalCanvas.qml" : "../components/SquiggleCanvas.qml"

        onLoaded: {
            if (item) {
                item.controller = root.controller;
                item.forceActiveFocus();
            }
        }
    }

    FocusScope {
        id: selectionStage
        anchors.fill: parent
        visible: root.controller.hasSelection
        focus: visible
        property rect area: root.controller.selection

        onVisibleChanged: {
            if (visible)
                forceActiveFocus();
        }

        AnnotationEditor {
            id: annotationEditor
            anchors.fill: parent
            controller: root.controller
            visible: root.annotationMode && root.controller.hasSelection
            ownsInput: root.ownsInput
            composerFocused: askComposer.textFocused
            bottomInset: askComposer.visible ? askComposer.height + 72 : 24
        }

        SelectionCrop {
            id: selectionCrop
            controller: root.controller
            visible: root.controller.hasSelection && !root.annotationMode
            enabled: !root.controller.qrPending
        }

        QrStatusDock {
            z: 30
            controller: root.controller
            visible: root.controller.action === "scan-qr" && root.controller.qrStatus.length > 0
            x: Math.max(8, (root.width - width) / 2)
            y: Math.max(8, root.height - height - 48)
            onAskGoogleRequested: root.controller.askGoogle("scan this qr code if found")
        }

        OcrActionDock {
            id: ocrDock
            z: 30
            controller: root.controller
            hasTextSelection: selectionCrop.hasTextSelection
            visible: root.controller.action === "ocr" && root.controller.hasSelection
            x: Math.max(8, (root.width - width) / 2)
            y: Math.max(8, root.height - height - 48)
            onActionRequested: function (action) {
                root.runOcrAction(action);
            }
        }

        AskGoogleComposer {
            id: askComposer
            z: 30
            visible: root.controller.action === "ask-google" && root.controller.hasSelection
            x: Math.max(8, (root.width - width) / 2)
            y: Math.max(8, root.height - height - 48)
            onSent: function (text) {
                root.controller.askGoogle(text);
            }
        }
    }

    CaptureExitHint {
        id: exitHint
        anchors.fill: parent
        active: root.visible
        z: 100
        onExitRequested: root.controller.cancel()
    }

    Shortcut {
        sequence: "Q"
        context: Qt.ApplicationShortcut
        enabled: root.ownsInput && root.visible && !root.annotationMode && !askComposer.visible
        onActivated: root.controller.cancel()
    }

    Shortcut {
        sequences: ["Ctrl+A", "Meta+A"]
        context: Qt.ApplicationShortcut
        enabled: root.ownsInput && root.visible && !root.annotationMode && root.controller.ocrVisible
        onActivated: selectionCrop.selectAllText()
    }

    Shortcut {
        sequences: ["Ctrl+C", "Ctrl+Insert"]
        context: Qt.ApplicationShortcut
        enabled: root.ownsInput && root.visible && !root.annotationMode && root.controller.ocrVisible
        onActivated: selectionCrop.copyTextSelection()
    }

    Shortcut {
        sequences: ["Ctrl+Shift+C", "Meta+Shift+C"]
        context: Qt.ApplicationShortcut
        enabled: root.ownsInput && root.visible && !root.annotationMode && root.controller.hasSelection && !askComposer.visible
        onActivated: root.controller.copyText()
    }
}
