pragma ComponentBehavior: Bound

import QtQuick

FloatingCapsule {
    id: root
    required property var controller
    property bool hasTextSelection: false
    property bool copied: false
    readonly property real availableWidth: Math.max(1, parent ? parent.width - 24 : 720)
    signal actionRequested(string action)

    width: Math.min(availableWidth, controller.ocrReady ? actions.width + 16 : statusContent.implicitWidth + 48)
    height: controller.ocrReady ? 56 : Math.max(56, statusContent.implicitHeight + 24)

    function actionEnabled(action) {
        if (!controller.ocrReady)
            return false;
        if (action === "selectAll")
            return controller.ocrVisible && controller.hasOcrText;
        if (action === "visibility")
            return true;
        return (action === "copy" || action === "search" || action === "translate") && hasTextSelection;
    }

    Behavior on width  {
        enabled: root.visible
        NumberAnimation {
            duration: 180
            easing.type: Easing.OutCubic
        }
    }
    Behavior on height  {
        enabled: root.visible
        NumberAnimation {
            duration: 180
            easing.type: Easing.OutCubic
        }
    }

    onVisibleChanged: {
        if (!visible) {
            copied = false;
            copyFeedback.stop();
        }
    }

    Connections {
        target: root.controller
        function onOcrTextCopied() {
            root.copied = true;
            copyFeedback.restart();
        }
    }
    Timer {
        id: copyFeedback
        interval: 1600
        onTriggered: root.copied = false
    }

    Row {
        id: statusContent
        anchors.centerIn: parent
        spacing: 10
        visible: !root.controller.ocrReady

        Canvas {
            id: spinner
            width: 22
            height: 22
            anchors.verticalCenter: parent.verticalCenter
            visible: !root.controller.ocrFailed
            onPaint: {
                const ctx = getContext("2d");
                ctx.clearRect(0, 0, width, height);
                ctx.strokeStyle = statusLabel.color;
                ctx.lineWidth = 2;
                ctx.lineCap = "round";
                ctx.beginPath();
                ctx.arc(width / 2, height / 2, width / 2 - 3, -Math.PI / 2, Math.PI);
                ctx.stroke();
            }
            RotationAnimator on rotation  {
                from: 0
                to: 360
                duration: 800
                loops: Animation.Infinite
                running: spinner.visible
            }
        }
        Text {
            id: statusLabel
            anchors.verticalCenter: parent.verticalCenter
            text: root.controller.ocrCancelling ? "Cancelling OCR" : root.controller.ocrFailed ? root.controller.ocrError : "Recognizing display text"
            width: Math.min(implicitWidth, Math.max(1, Math.min(720, root.availableWidth) - 48 - (root.controller.ocrFailed ? retryButton.width + statusContent.spacing : 22 + statusContent.spacing)))
            wrapMode: Text.Wrap
            color: "#1c1c1e"
            font.family: Qt.platform.os === "osx" ? "SF Pro Text" : "Segoe UI"
            font.pixelSize: 14
        }
        Rectangle {
            id: retryButton
            anchors.verticalCenter: parent.verticalCenter
            visible: root.controller.ocrFailed
            width: retryLabel.implicitWidth + 24
            height: 32
            radius: 8
            color: retryMouse.pressed ? "#20000000" : retryMouse.containsMouse ? "#10000000" : "transparent"
            Text {
                id: retryLabel
                anchors.centerIn: parent
                text: "Retry"
                color: "#1c1c1e"
                font.family: Qt.platform.os === "osx" ? "SF Pro Text" : "Segoe UI"
                font.pixelSize: 14
            }
            MouseArea {
                id: retryMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: root.actionRequested("retry")
            }
        }
    }

    Flickable {
        id: viewport
        anchors.fill: parent
        anchors.margins: 8
        contentWidth: Math.max(width, actions.width)
        contentHeight: height
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        flickableDirection: Flickable.HorizontalFlick
        visible: opacity > 0
        enabled: root.controller.ocrReady
        opacity: root.controller.ocrReady ? 1 : 0

        Behavior on opacity  {
            NumberAnimation {
                duration: 180
                easing.type: Easing.OutCubic
            }
        }

        Row {
            id: actions
            x: Math.max(0, (viewport.width - width) / 2)
            height: viewport.height
            Repeater {
                model: ["selectAll", "copy", "search", "translate", "visibility"]
                delegate: Item {
                    id: button
                    required property string modelData
                    required property int index
                    readonly property bool active: root.actionEnabled(modelData)
                    width: Math.max(label.implicitWidth, reservedLabel.width) + 28
                    height: viewport.height

                    TextMetrics {
                        id: reservedLabel
                        font: label.font
                        text: button.modelData === "copy" ? "Copied" : button.modelData === "visibility" ? "Show OCR" : label.text
                    }
                    Rectangle {
                        anchors.fill: parent
                        anchors.margins: 2
                        radius: 8
                        color: buttonMouse.pressed ? "#20000000" : buttonMouse.containsMouse ? "#10000000" : "transparent"
                    }
                    Text {
                        id: label
                        anchors.centerIn: parent
                        text: button.modelData === "selectAll" ? "Select all" : button.modelData === "copy" ? (root.copied ? "Copied" : "Copy") : button.modelData === "search" ? "Search" : button.modelData === "translate" ? "Open in Translate" : root.controller.ocrVisible ? "Hide OCR" : "Show OCR"
                        color: "#1c1c1e"
                        opacity: button.active ? 1 : 0.35
                        font.family: Qt.platform.os === "osx" ? "SF Pro Text" : "Segoe UI"
                        font.pixelSize: 14
                    }
                    Rectangle {
                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        width: 1
                        height: 18
                        color: "#24000000"
                        visible: button.index < 4
                    }
                    MouseArea {
                        id: buttonMouse
                        anchors.fill: parent
                        enabled: button.active
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.actionRequested(button.modelData)
                    }
                }
            }
        }
    }
}
