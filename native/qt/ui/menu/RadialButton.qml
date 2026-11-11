import QtQuick
import QtQuick.Window
import RadialMenu.Native 1.0

Item {
    id: button

    required property string label
    required property string action
    required property string iconName
    required property real angle
    required property real outerRadius
    required property real revealOpacity
    required property real renderScale
    required property color hoverColor
    required property color backgroundColor
    required property real hoverAlpha
    readonly property bool hovered: pointer.containsMouse
    readonly property bool down: pointer.pressed
    signal activated()

    width: 1000
    height: 1000
    activeFocusOnTab: true
    RadialPetal {
        id: petal
        anchors.fill: parent
        angle: button.angle
        outerRadius: button.outerRadius
        contentSize: Qt.size(content.width, content.height)
        fillColor: button.down ? Qt.rgba(button.hoverColor.r, button.hoverColor.g, button.hoverColor.b, Math.min(1, button.hoverAlpha + 0.1))
                              : button.hovered || button.activeFocus ? Qt.rgba(button.hoverColor.r, button.hoverColor.g, button.hoverColor.b, button.hoverAlpha)
                              : button.backgroundColor

        Behavior on fillColor {
            ColorAnimation { duration: 140 }
        }
    }

    Accessible.role: Accessible.Button
    Accessible.name: action
    Accessible.onPressAction: button.activated()
    Keys.onSpacePressed: button.activated()
    Keys.onReturnPressed: button.activated()
    Keys.onEnterPressed: button.activated()

    MouseArea {
        id: pointer
        anchors.fill: parent
        containmentMask: petal
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: button.activated()
    }

    Image {
        id: content
        opacity: button.revealOpacity
        x: petal.contentRect.x + (petal.contentRect.width - width) / 2
        y: petal.contentRect.y + (petal.contentRect.height - height) / 2
        width: 112
        height: 104
        scale: Math.min(1, petal.contentRect.width / width)
        source: "icons/" + button.iconName + ".svg"
        sourceSize.width: Math.max(1, Math.ceil(width * button.renderScale * scale * Screen.devicePixelRatio * 2))
        sourceSize.height: Math.max(1, Math.ceil(height * button.renderScale * scale * Screen.devicePixelRatio * 2))
        fillMode: Image.PreserveAspectFit
        smooth: true
        mipmap: true
    }
}
