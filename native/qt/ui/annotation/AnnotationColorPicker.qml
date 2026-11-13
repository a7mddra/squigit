pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls

Popup {
    id: root
    required property var document
    property bool selectionOnly: false
    property bool colorChosen: false
    property real hue: 0
    property real saturation: 1
    property real brightness: 1
    readonly property bool sizeOnly: !selectionOnly && document.selectionCount < 2 && !document.colorTool
    readonly property bool showSize: !selectionOnly && document.selectionCount < 2 && !document.solidTool
    width: sizeOnly ? 218 : showSize ? 226 : 184
    height: sizeOnly ? 32 : 212
    z: 200
    padding: 0
    focus: true
    modal: false
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
    onOpened: root.document.beginFormatting()
    onClosed: root.document.finishFormatting()
    background: Item {
    }

    function showAt(px, py, groupOnly, aboveAnchor) {
        selectionOnly = groupOnly;
        colorChosen = !document.mixedColors;
        hue = Math.max(0, document.inkColor.hsvHue);
        saturation = document.inkColor.hsvSaturation;
        brightness = document.inkColor.hsvValue;
        x = Math.max(12, Math.min(px, parent.width - width - 12));
        y = Math.max(12, Math.min(aboveAnchor ? py - height - 10 : py, parent.height - height - 12));
        open();
    }
    function applyColor() {
        colorChosen = true;
        document.inkColor = Qt.hsva(hue, saturation, brightness, 1);
    }

    contentItem: Item {
        Rectangle {
            id: spectrum
            visible: !root.sizeOnly
            width: 184
            height: 150
            radius: 5
            color: Qt.hsva(root.hue, 1, 1, 1)
            Rectangle {
                anchors.fill: parent
                radius: 5
                gradient: Gradient {
                    orientation: Gradient.Horizontal
                    GradientStop {
                        position: 0
                        color: "white"
                    }
                    GradientStop {
                        position: 1
                        color: "transparent"
                    }
                }
            }
            Rectangle {
                anchors.fill: parent
                radius: 5
                gradient: Gradient {
                    GradientStop {
                        position: 0
                        color: "transparent"
                    }
                    GradientStop {
                        position: 1
                        color: "black"
                    }
                }
            }
            Rectangle {
                x: root.saturation * spectrum.width - 5
                y: (1 - root.brightness) * spectrum.height - 5
                width: 10
                height: 10
                radius: 5
                color: "transparent"
                border.width: 2
                border.color: "white"
                visible: root.colorChosen
            }
            MouseArea {
                anchors.fill: parent
                function change(mouse) {
                    root.saturation = Math.max(0, Math.min(1, mouse.x / width));
                    root.brightness = 1 - Math.max(0, Math.min(1, mouse.y / height));
                    root.applyColor();
                }
                onPressed: function (mouse) {
                    change(mouse);
                }
                onPositionChanged: function (mouse) {
                    if (pressed)
                        change(mouse);
                }
            }
        }
        Slider {
            id: hueSlider
            visible: !root.sizeOnly
            y: spectrum.height + 8
            width: spectrum.width
            height: 16
            padding: 0
            from: 0
            to: 1
            value: root.hue
            onMoved: {
                root.hue = value;
                root.applyColor();
            }
            background: Rectangle {
                y: (hueSlider.height - height) / 2
                width: hueSlider.width
                height: 8
                radius: 4
                gradient: Gradient {
                    orientation: Gradient.Horizontal
                    GradientStop {
                        position: 0
                        color: "#ff0000"
                    }
                    GradientStop {
                        position: 0.167
                        color: "#ffff00"
                    }
                    GradientStop {
                        position: 0.333
                        color: "#00ff00"
                    }
                    GradientStop {
                        position: 0.5
                        color: "#00ffff"
                    }
                    GradientStop {
                        position: 0.667
                        color: "#0000ff"
                    }
                    GradientStop {
                        position: 0.833
                        color: "#ff00ff"
                    }
                    GradientStop {
                        position: 1
                        color: "#ff0000"
                    }
                }
            }
            handle: Rectangle {
                x: hueSlider.visualPosition * (hueSlider.width - width)
                y: (hueSlider.height - height) / 2
                width: 14
                height: 14
                radius: 7
                color: "transparent"
                border.width: 2
                border.color: "white"
            }
        }
        TextField {
            id: hexInput
            visible: !root.sizeOnly
            y: hueSlider.y + hueSlider.height + 8
            width: spectrum.width
            height: 30
            text: root.colorChosen ? root.document.inkColor.toString().toUpperCase() : ""
            placeholderText: "Mixed colors"
            color: "#f5f5f5"
            placeholderTextColor: "#b0b0b0"
            selectedTextColor: "white"
            selectionColor: "#666666"
            font.pixelSize: 12
            selectByMouse: true
            maximumLength: 7
            background: Rectangle {
                color: "#252525"
                radius: 5
                border.color: "#555555"
            }
            onEditingFinished: {
                if (/^#[0-9a-fA-F]{6}$/.test(text)) {
                    root.document.inkColor = text;
                    root.colorChosen = true;
                    root.hue = Math.max(0, root.document.inkColor.hsvHue);
                    root.saturation = root.document.inkColor.hsvSaturation;
                    root.brightness = root.document.inkColor.hsvValue;
                }
            }
        }
        AnnotationSizeControl {
            document: root.document
            x: root.sizeOnly ? 0 : spectrum.width + 10
            width: root.sizeOnly ? root.width : 36
            height: root.height
            orientation: root.sizeOnly ? Qt.Horizontal : Qt.Vertical
            visible: root.showSize
        }
    }
}
