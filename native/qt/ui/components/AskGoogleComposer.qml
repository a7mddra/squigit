pragma ComponentBehavior: Bound

import QtQuick
import "../annotation"

FloatingCapsule {
    id: root

    readonly property bool textFocused: textArea.activeFocus
    readonly property int composerWidth: 560
    readonly property int composerHeight: 56
    readonly property int maxBodyHeight: 200
    readonly property bool hasText: textArea.text.trim().length > 0

    signal sent(string text)

    width: Math.min(composerWidth, parent ? parent.width - 24 : composerWidth)
    height: Math.min(root.maxBodyHeight, Math.max(root.composerHeight, textArea.contentHeight + root.capRadius * 2 - textArea.font.pixelSize))
    z: 10
    onVisibleChanged: {
        if (visible) {
            Qt.callLater(function () {
                    textArea.forceActiveFocus();
                });
        } else {
            textArea.text = "";
        }
    }

    Behavior on height  {
        NumberAnimation {
            id: heightAnim
            duration: 180
            easing.type: Easing.OutCubic
        }
    }

    function dismissContextMenu() {
        if (!composerMenu.visible)
            return false;
        composerMenu.close();
        return true;
    }

    TextContextMenu {
        id: composerMenu
        parent: root.parent
    }

    function send() {
        if (!hasText)
            return;
        root.sent(textArea.text);
        textArea.text = "";
    }

    Item {
        id: body
        anchors.fill: parent

        Flickable {
            id: flickable
            anchors {
                left: parent.left
                right: sendButton.left
                top: parent.top
                bottom: parent.bottom
                leftMargin: root.capRadius
                rightMargin: 18
                topMargin: root.capRadius - textArea.font.pixelSize / 2 - 2
                bottomMargin: root.capRadius - textArea.font.pixelSize / 2 - 2
            }
            contentWidth: width
            contentHeight: textArea.contentHeight
            clip: true
            flickableDirection: Flickable.VerticalFlick
            boundsBehavior: Flickable.StopAtBounds

            onContentHeightChanged: {
                if (contentHeight <= height) {
                    contentY = 0;
                } else {
                    ensureCursorVisible();
                }
            }

            function ensureCursorVisible() {
                var cursorRect = textArea.cursorRectangle;
                if (!cursorRect)
                    return;
                var yInFlickable = cursorRect.y;
                if (yInFlickable < contentY) {
                    contentY = Math.max(0, yInFlickable - 4);
                } else if (yInFlickable + cursorRect.height > contentY + height) {
                    contentY = Math.min(contentHeight - height, yInFlickable + cursorRect.height - height + 4);
                }
            }

            TextEdit {
                id: textArea
                width: flickable.width
                color: "#1c1c1e"
                selectionColor: "#3584e4"
                selectedTextColor: "#ffffff"
                font.family: Qt.platform.os === "osx" ? "SF Pro Text" : "Segoe UI"
                font.pixelSize: 16
                font.weight: Font.Normal
                wrapMode: TextEdit.Wrap
                selectByMouse: true
                persistentSelection: true
                textFormat: TextEdit.PlainText
                focus: true
                activeFocusOnTab: true
                renderType: Text.QtRendering
                verticalAlignment: TextEdit.AlignVCenter

                MouseArea {
                    anchors.fill: parent
                    acceptedButtons: Qt.RightButton
                    hoverEnabled: true
                    cursorShape: Qt.IBeamCursor
                    onPressed: function (mouse) {
                        if (textArea.selectedText.length === 0)
                            textArea.cursorPosition = textArea.positionAt(mouse.x, mouse.y);
                        const point = mapToItem(root.parent, mouse.x, mouse.y);
                        composerMenu.showFor(textArea, point.x, point.y, false);
                    }
                }

                onCursorRectangleChanged: flickable.ensureCursorVisible()

                Text {
                    anchors.fill: parent
                    text: "Ask anything..."
                    color: Qt.rgba(textArea.color.r, textArea.color.g, textArea.color.b, 0.8)
                    font: textArea.font
                    visible: textArea.text.length === 0
                    verticalAlignment: Text.AlignVCenter
                    renderType: Text.QtRendering
                }

                Keys.onPressed: function (event) {
                    if ((event.key === Qt.Key_Return || event.key === Qt.Key_Enter) && !(event.modifiers & Qt.ShiftModifier)) {
                        event.accepted = true;
                        root.send();
                    }
                }
            }
        }

        Item {
            id: scrollBarTrack
            anchors {
                right: sendButton.left
                rightMargin: 6
                top: flickable.top
                bottom: flickable.bottom
            }
            width: 4

            readonly property bool confirmedToShow: (body.height >= root.maxBodyHeight - 1) && !heightAnim.running && (flickable.contentHeight > flickable.height)

            Rectangle {
                anchors.fill: parent
                radius: width / 2
                color: scrollBarTrack.confirmedToShow ? Qt.rgba(0, 0, 0, 0.06) : "transparent"

                Behavior on color  {
                    ColorAnimation {
                        duration: 180
                    }
                }
            }

            Rectangle {
                id: scrollThumb
                width: parent.width
                radius: width / 2
                color: scrollBarTrack.confirmedToShow ? (scrollMouseArea.containsMouse || flickable.moving || flickable.flicking ? Qt.rgba(0, 0, 0, 0.55) : Qt.rgba(0, 0, 0, 0.35)) : "transparent"

                Behavior on color  {
                    ColorAnimation {
                        duration: 180
                    }
                }

                readonly property real maxScroll: flickable.contentHeight - flickable.height
                readonly property real progress: maxScroll > 0 ? Math.max(0, Math.min(1, flickable.contentY / maxScroll)) : 0
                readonly property real availableThumbHeight: flickable.contentHeight > 0 ? (flickable.height / flickable.contentHeight) * scrollBarTrack.height : scrollBarTrack.height

                height: Math.max(16, Math.min(scrollBarTrack.height, availableThumbHeight))
                y: progress * (scrollBarTrack.height - height)
            }

            MouseArea {
                id: scrollMouseArea
                anchors.fill: parent
                anchors.margins: -4
                enabled: scrollBarTrack.confirmedToShow
                hoverEnabled: scrollBarTrack.confirmedToShow
                cursorShape: Qt.PointingHandCursor
                preventStealing: true

                function updatePosition(mouseY) {
                    var availableTrack = scrollBarTrack.height - scrollThumb.height;
                    if (availableTrack > 0) {
                        var targetY = Math.max(0, Math.min(availableTrack, mouseY - scrollThumb.height / 2));
                        var progress = targetY / availableTrack;
                        flickable.contentY = progress * (flickable.contentHeight - flickable.height);
                    }
                }

                onPressed: function (mouse) {
                    updatePosition(mouse.y);
                }
                onPositionChanged: function (mouse) {
                    if (pressed)
                        updatePosition(mouse.y);
                }
            }
        }

        Rectangle {
            id: sendButton
            anchors {
                right: parent.right
                bottom: parent.bottom
                rightMargin: (root.composerHeight - width) / 2
                bottomMargin: (root.composerHeight - height) / 2
            }
            width: 32
            height: 32
            radius: 16
            color: root.hasText ? "#007AFF" : Qt.rgba(0.45, 0.45, 0.50, 0.25)
            antialiasing: true

            Behavior on color  {
                ColorAnimation {
                    duration: 200
                }
            }

            Canvas {
                anchors.centerIn: parent
                width: 16
                height: 16

                onPaint: {
                    var ctx = getContext("2d");
                    ctx.clearRect(0, 0, width, height);
                    ctx.strokeStyle = "#ffffff";
                    ctx.lineWidth = 2.0;
                    ctx.lineCap = "round";
                    ctx.lineJoin = "round";
                    ctx.beginPath();
                    ctx.moveTo(8, 14);
                    ctx.lineTo(8, 3);
                    ctx.stroke();
                    ctx.beginPath();
                    ctx.moveTo(3, 7);
                    ctx.lineTo(8, 2);
                    ctx.lineTo(13, 7);
                    ctx.stroke();
                }
            }

            MouseArea {
                id: sendMouse
                anchors.fill: parent
                hoverEnabled: true
                enabled: root.hasText
                cursorShape: root.hasText ? Qt.PointingHandCursor : Qt.ArrowCursor
                onPressed: sendButton.scale = 0.85
                onReleased: sendButton.scale = 1.0
                onCanceled: sendButton.scale = 1.0
                onClicked: root.send()
            }

            AnnotationToolTip {
                text: "Send · Enter"
                visible: sendMouse.containsMouse
            }

            Behavior on scale  {
                NumberAnimation {
                    duration: 100
                    easing.type: Easing.OutCubic
                }
            }
        }
    }
}
