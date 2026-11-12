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
    property int hoveredIndex: -1
    property int keyboardIndex: -1
    property bool keyboardNavigation: false
    readonly property int highlightedIndex: keyboardNavigation ? keyboardIndex : hoveredIndex
    readonly property string previewLabel: {
        const button = highlightedIndex >= 0 ? petals.itemAt(highlightedIndex) : null
        return button ? button.label : ""
    }

    function moveFocus(step) {
        if (!active || !visible || petals.count === 0)
            return
        const currentIndex = keyboardNavigation ? keyboardIndex : hoveredIndex
        const nextIndex = currentIndex < 0
            ? (step > 0 ? 0 : petals.count - 1)
            : (currentIndex + step + petals.count) % petals.count
        keyboardNavigation = true
        keyboardIndex = nextIndex
        wheel.forceActiveFocus()
    }

    function pointAt(index) {
        hoverClear.stop()
        hoveredIndex = index
        keyboardIndex = index
        keyboardNavigation = false
    }

    function clearPointAt(index) {
        if (hoveredIndex === index)
            hoverClear.restart()
    }

    onHighlightedIndexChanged: {
        if (highlightedIndex >= 0)
            menuActions.playTick()
    }

    Timer {
        id: hoverClear
        interval: 60
        onTriggered: {
            if (!window.keyboardNavigation) {
                window.hoveredIndex = -1
            }
        }
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
        openingPending = visible
        opacity = visible ? 1 : 0
        if (visible) {
            openingProgress = 0.7
            wheel.forceActiveFocus()
        } else {
            tickDelay.stop()
            hoverClear.stop()
            hoveredIndex = -1
            keyboardIndex = -1
            keyboardNavigation = false
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
        enabled: window.visible && window.active
        onActivated: menuActions.hide()
    }
    Shortcut {
        sequence: "Ctrl+Q"
        enabled: window.visible && window.active
        onActivated: menuActions.hide()
    }

    Window {
        id: hoverBadge
        objectName: "hoverBadge"
        readonly property real uiScale: window.diameter / 1000
        transientParent: window
        title: "Squigit"
        flags: Qt.Tool | Qt.FramelessWindowHint | Qt.NoDropShadowWindowHint
               | Qt.WindowTransparentForInput | Qt.WindowDoesNotAcceptFocus
        color: "transparent"
        visible: window.visible && window.openingProgress === 1 && window.previewLabel.length > 0
        onVisibleChanged: opacity = visible ? 1 : 0
        width: Math.floor(window.innerRadius * 1.8 * uiScale)
        height: Math.ceil(hoverText.implicitHeight + 32 * uiScale)
        x: window.x + Math.round((window.width - width) / 2)
        y: window.y + Math.round((window.height - height) / 2)

        Rectangle {
            id: badgeSurface
            anchors.centerIn: parent
            width: Math.ceil(Math.min(hoverText.implicitWidth + 48 * hoverBadge.uiScale,
                                     hoverBadge.width))
            height: parent.height
            radius: 24 * hoverBadge.uiScale
            color: window.petalColor
            antialiasing: true

            Behavior on width {
                NumberAnimation {
                    duration: 120
                    easing.type: Easing.OutCubic
                }
            }

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
        focus: true

        Keys.priority: Keys.BeforeItem
        Keys.onPressed: function(event) {
            if (!window.active || (event.modifiers & (Qt.ControlModifier | Qt.AltModifier | Qt.MetaModifier)))
                return
            let step = 0
            switch (event.key) {
            case Qt.Key_Right:
            case Qt.Key_Down:
                step = 1
                break
            case Qt.Key_Left:
            case Qt.Key_Up:
            case Qt.Key_Backtab:
                step = -1
                break
            case Qt.Key_Tab:
                step = event.modifiers & Qt.ShiftModifier ? -1 : 1
                break
            default:
                return
            }
            event.accepted = true
            window.moveFocus(step)
        }

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
            id: petals
            model: [
                { label: "Circle to Search", action: "Circle to Search", icon: "circle-to-search", angle: 0 },
                { label: "OCR", action: "OCR", icon: "ocr", angle: 72 },
                { label: "Snapshot", action: "Snapshot", icon: "snapshot", angle: 144 },
                { label: "Ask Google", action: "Ask Google", icon: "ask-google", angle: 216 },
                { label: "Scan QR", action: "Scan QR", icon: "scan-qr", angle: 288 }
            ]

            RadialButton {
                id: radialButton
                required property var modelData
                required property int index
                highlighted: window.highlightedIndex === index
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
                    if (hovered)
                        window.pointAt(index)
                    else
                        window.clearPointAt(index)
                }
                onPointerMoved: window.pointAt(index)
                onActivated: menuActions.press(action)
            }
        }
    }
}
