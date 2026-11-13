pragma ComponentBehavior: Bound

import QtQuick
import Qt5Compat.GraphicalEffects

Item {
    id: root
    default property alias content: contentHost.data
    property real capRadius: 28
    readonly property real startBlurRadius: 24
    property real openScale: 1.15
    property real openOpacity: 0
    property real openBlur: startBlurRadius

    width: Math.min(560, parent ? parent.width - 24 : 560)
    height: 56
    scale: openScale
    opacity: openOpacity
    transformOrigin: Item.Center

    layer.enabled: root.visible && openBlur > 0
    layer.effect: GaussianBlur {
        radius: root.openBlur
        deviation: (root.startBlurRadius + 1) / 3.3333
        samples: 49
        transparentBorder: true
        cached: false
    }

    function restartOpenAnimation() {
        openAnimation.stop();
        openScale = 1.15;
        openOpacity = 0;
        openBlur = startBlurRadius;
        if (visible)
            openAnimation.start();
    }

    Connections {
        target: root
        function onVisibleChanged() {
            root.restartOpenAnimation();
        }
    }
    Component.onCompleted: restartOpenAnimation()

    ParallelAnimation {
        id: openAnimation
        NumberAnimation {
            target: root
            property: "openScale"
            from: 1.15
            to: 1.0
            duration: 700
            easing.type: Easing.BezierSpline
            easing.bezierCurve: [0.16, 1, 0.3, 1, 1, 1]
        }
        NumberAnimation {
            target: root
            property: "openOpacity"
            from: 0
            to: 1
            duration: 700
            easing.type: Easing.BezierSpline
            easing.bezierCurve: [0.16, 1, 0.3, 1, 1, 1]
        }
        NumberAnimation {
            target: root
            property: "openBlur"
            from: root.startBlurRadius
            to: 0
            duration: 700
            easing.type: Easing.BezierSpline
            easing.bezierCurve: [0.16, 1, 0.3, 1, 1, 1]
        }
    }

    Rectangle {
        anchors.fill: parent
        radius: root.capRadius
        color: Qt.rgba(0.96, 0.96, 0.98, 0.95)
        border.width: 1
        border.color: Qt.rgba(1, 1, 1, 0.6)
        antialiasing: true
        Rectangle {
            anchors.fill: parent
            anchors.margins: 1
            radius: parent.radius - 1
            color: "transparent"
            border.width: 1
            border.color: Qt.rgba(1, 1, 1, 0.35)
            antialiasing: true
        }
        Rectangle {
            anchors.fill: parent
            radius: parent.radius
            gradient: Gradient {
                GradientStop {
                    position: 0
                    color: Qt.rgba(1, 1, 1, 0.18)
                }
                GradientStop {
                    position: 0.45
                    color: Qt.rgba(1, 1, 1, 0)
                }
                GradientStop {
                    position: 1
                    color: Qt.rgba(0, 0, 0, 0.06)
                }
            }
            antialiasing: true
        }
    }
    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.AllButtons
        hoverEnabled: true
        cursorShape: Qt.ArrowCursor
    }
    Item {
        id: contentHost
        anchors.fill: parent
    }
    Rectangle {
        anchors.fill: parent
        anchors.margins: -2
        radius: root.capRadius + 2
        z: -1
        color: "transparent"
        border.width: 12
        border.color: Qt.rgba(0, 0, 0, 0.08)
        antialiasing: true
    }
    Rectangle {
        anchors.fill: parent
        anchors.margins: -6
        radius: root.capRadius + 6
        z: -2
        color: "transparent"
        border.width: 8
        border.color: Qt.rgba(0, 0, 0, 0.03)
        antialiasing: true
    }
}
