import QtQuick
import QtQuick.Window

Window {
    id: window

    readonly property int diameter: menuConfig.diameter
    readonly property real circleAlpha: menuConfig["circle-opacity"]
    readonly property real buttonAlpha: menuConfig["button-opacity"]
    readonly property real hoverAlpha: menuConfig["hover-opacity"]
    readonly property color hoverColor: "#ffe3a4"
    readonly property color petalColor: Qt.rgba(1, 0.94, 0.85, buttonAlpha)
    readonly property real innerRadius: menuConfig["inner-radius"]
    readonly property real petalOuterRadius: menuConfig["petal-outer-radius"]
    readonly property real outerRim: menuConfig["outer-rim"]
    property real openingProgress: 0.7
    property bool openingPending: false
    property Item hoveredButton: null
    property Item focusedButton: null
    readonly property string previewLabel: {
        const button = hoveredButton || focusedButton
        return button ? button.label : ""
    }

    width: diameter + 24
    height: width
    minimumWidth: width
    maximumWidth: width
    minimumHeight: height
    maximumHeight: height
    title: "Squigit"
    visible: false
    onClosing: function(close) {
        close.accepted = false
        menuActions.hide()
    }
    color: "transparent"
    flags: Qt.Window | Qt.FramelessWindowHint | Qt.NoDropShadowWindowHint

    onVisibleChanged: {
        openingAnimation.stop()
        openingProgress = 0.7
        openingPending = visible
        if (!visible) {
            hoveredButton = null
            focusedButton = null
        }
    }

    onFrameSwapped: {
        if (!openingPending) return
        openingPending = false
        openingAnimation.start()
    }

    NumberAnimation {
        id: openingAnimation
        target: window
        property: "openingProgress"
        from: 0.7
        to: 1
        duration: 160
        easing.type: Easing.BezierSpline
        easing.bezierCurve: [0.16, 1, 0.3, 1, 1, 1]
    }

    Shortcut {
        sequence: "Escape"
        enabled: window.visible
        onActivated: menuActions.hide()
    }
    Shortcut {
        sequence: "Ctrl+Q"
        enabled: window.visible
        onActivated: menuActions.hide()
    }

    Window {
        id: hoverBadge
        readonly property real uiScale: window.diameter / 1000
        transientParent: window
        title: "Squigit"
        flags: Qt.ToolTip | Qt.FramelessWindowHint | Qt.NoDropShadowWindowHint
               | Qt.WindowTransparentForInput | Qt.WindowDoesNotAcceptFocus
        color: "transparent"
        visible: window.visible && window.openingProgress === 1 && window.previewLabel.length > 0
        width: Math.ceil(window.innerRadius * 2 * uiScale)
        height: width
        x: window.x + Math.round((window.width - width) / 2)
        y: window.y + Math.round((window.height - height) / 2)

        Rectangle {
            anchors.centerIn: parent
            width: Math.min(hoverText.implicitWidth + 48 * hoverBadge.uiScale,
                            window.innerRadius * 1.8 * hoverBadge.uiScale)
            height: hoverText.implicitHeight + 32 * hoverBadge.uiScale
            radius: 24 * hoverBadge.uiScale
            color: window.petalColor
            antialiasing: true

            Text {
                id: hoverText
                anchors.centerIn: parent
                width: parent.width - 32 * hoverBadge.uiScale
                text: window.previewLabel
                color: "#121310"
                font.family: Qt.platform.os === "osx" ? "Helvetica Neue" : "Arial"
                font.pixelSize: 32 * hoverBadge.uiScale
                font.weight: Font.DemiBold
                font.letterSpacing: 0.8 * hoverBadge.uiScale
                fontSizeMode: Text.HorizontalFit
                minimumPixelSize: Math.max(1, 20 * hoverBadge.uiScale)
                wrapMode: Text.NoWrap
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
                renderType: Text.QtRendering
            }
        }
    }

    Item {
        id: wheel
        anchors.centerIn: parent
        width: 1000
        height: 1000
        scale: window.diameter / 1000

        Rectangle {
            anchors.centerIn: parent
            width: 2 * (window.petalOuterRadius * window.openingProgress + window.outerRim)
            height: width
            radius: width / 2
            color: "transparent"
            border.width: width / 2 - window.innerRadius
            border.color: Qt.rgba(1, 0.90, 0.76, window.circleAlpha)
            antialiasing: true
        }

        MouseArea {
            anchors.fill: parent
            containmentMask: QtObject {
                function contains(position: point): bool {
                    const distance = Math.pow(position.x - 500, 2) + Math.pow(position.y - 500, 2);
                    return distance >= window.innerRadius * window.innerRadius && distance <= 500 * 500;
                }
            }
            cursorShape: pressed ? Qt.ClosedHandCursor : Qt.OpenHandCursor
            onPressed: window.startSystemMove()
        }

        Repeater {
            model: [
                { label: "CIRCLE TO\nSEARCH", action: "Circle to Search", icon: "circle-to-search", angle: 0 },
                { label: "OCR", action: "OCR", icon: "ocr", angle: 72 },
                { label: "SNAPSHOT", action: "Snapshot", icon: "snapshot", angle: 144 },
                { label: "ASK GOOGLE", action: "Ask Google", icon: "ask-google", angle: 216 },
                { label: "SCAN QR", action: "Scan QR", icon: "scan-qr", angle: 288 }
            ]

            RadialButton {
                id: radialButton
                required property var modelData
                label: modelData.label
                action: modelData.action
                iconName: modelData.icon
                angle: modelData.angle
                outerRadius: window.petalOuterRadius * window.openingProgress
                revealOpacity: Math.max(0, Math.min(1, (window.openingProgress - 0.9) / 0.1))
                renderScale: window.diameter / 1000
                hoverColor: window.hoverColor
                backgroundColor: window.petalColor
                hoverAlpha: window.hoverAlpha
                onHoveredChanged: {
                    if (hovered) window.hoveredButton = radialButton
                    else if (window.hoveredButton === radialButton) window.hoveredButton = null
                }
                onActiveFocusChanged: {
                    if (activeFocus) window.focusedButton = radialButton
                    else if (window.focusedButton === radialButton) window.focusedButton = null
                }
                onActivated: menuActions.press(action)
            }
        }
    }
}
