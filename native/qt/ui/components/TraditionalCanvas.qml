import QtQuick
import Qt5Compat.GraphicalEffects

/**
 * Traditional selection canvas.
 *
 * Features:
 * - Sharp corner at cursor position, rounded elsewhere
 * - 65% brightness dim overlay outside selection
 * - Gradient stroke for shine/sparkle effect
 * - Smooth outer glow
 */

Item {
    id: root
    anchors.fill: parent
    focus: true

    property var controller

    property point startPoint: Qt.point(0, 0)
    property point endPoint: Qt.point(0, 0)
    property point pendingEndPoint: Qt.point(0, 0)
    property bool isDrawing: false
    property bool hasSelection: false
    property bool hasPendingEndPoint: false
    property bool repaintPending: false

    readonly property real selX: Math.min(startPoint.x, endPoint.x)
    readonly property real selY: Math.min(startPoint.y, endPoint.y)
    readonly property real selW: Math.abs(endPoint.x - startPoint.x)
    readonly property real selH: Math.abs(endPoint.y - startPoint.y)
    readonly property int dragFrameIntervalMs: 16
    readonly property real minDragDistance: 0.5

    function queueRepaint() {
        root.repaintPending = true
        if (!paintTimer.running) {
            paintTimer.start()
        }
    }

    function setPendingEndPoint(x, y, forceFlush) {
        var refPoint = root.hasPendingEndPoint ? root.pendingEndPoint : root.endPoint
        var dx = x - refPoint.x
        var dy = y - refPoint.y
        var minDistanceSq = root.minDragDistance * root.minDragDistance

        if (!forceFlush && (dx * dx + dy * dy) < minDistanceSq) {
            return
        }

        root.pendingEndPoint = Qt.point(x, y)
        root.hasPendingEndPoint = true

        if (forceFlush) {
            root.flushPendingEndPoint()
        } else {
            root.queueRepaint()
        }
    }

    function flushPendingEndPoint() {
        if (!root.hasPendingEndPoint) {
            return false
        }

        var pending = root.pendingEndPoint
        var changed = root.endPoint.x !== pending.x || root.endPoint.y !== pending.y
        if (changed) {
            root.endPoint = Qt.point(pending.x, pending.y)
        }
        root.hasPendingEndPoint = false
        return changed
    }

    Timer {
        id: paintTimer
        interval: root.dragFrameIntervalMs
        repeat: true
        running: false

        onTriggered: {
            var changed = root.flushPendingEndPoint()
            var needsPaint = root.repaintPending || changed
            root.repaintPending = false

            if (!needsPaint) {
                stop()
                return
            }

            dimCanvas.requestPaint()
            selectionBorderCanvas.requestPaint()
        }
    }

    Canvas {
        id: dimCanvas
        anchors.fill: parent
        renderStrategy: Canvas.Threaded
        opacity: 0

        NumberAnimation on opacity {
            from: 0; to: 1
            duration: 200
            running: true
            easing.type: Easing.OutQuad
        }

        onPaint: {
            var ctx = getContext("2d")
            ctx.reset()

            ctx.fillStyle = Qt.rgba(0, 0, 0, 0.35)
            ctx.fillRect(0, 0, width, height)

            if ((root.isDrawing || root.hasSelection) && root.selW > 0 && root.selH > 0) {
                ctx.globalCompositeOperation = "destination-out"
                ctx.fillStyle = "white"
                root.drawSelectionPath(ctx, root.selX, root.selY, root.selW, root.selH)
                ctx.fill()
            }
        }
    }

    function drawSelectionPath(ctx, x, y, w, h) {
        if (w <= 0 || h <= 0) return

        var baseRadius = Math.min(24, Math.min(w, h) / 2)

        var tl = baseRadius, tr = baseRadius, br = baseRadius, bl = baseRadius

        if (root.endPoint.x >= root.startPoint.x) {
            if (root.endPoint.y >= root.startPoint.y) br = 0
            else tr = 0
        } else {
            if (root.endPoint.y >= root.startPoint.y) bl = 0
            else tl = 0
        }

        ctx.beginPath()

        ctx.moveTo(x + tl, y)
        ctx.lineTo(x + w - tr, y)
        if (tr > 0) ctx.quadraticCurveTo(x + w, y, x + w, y + tr)

        ctx.lineTo(x + w, y + h - br)
        if (br > 0) ctx.quadraticCurveTo(x + w, y + h, x + w - br, y + h)

        ctx.lineTo(x + bl, y + h)
        if (bl > 0) ctx.quadraticCurveTo(x, y + h, x, y + h - bl)

        ctx.lineTo(x, y + tl)
        if (tl > 0) ctx.quadraticCurveTo(x, y, x + tl, y)

        ctx.closePath()
    }

    Glow {
        anchors.fill: selectionBorderCanvas
        source: selectionBorderCanvas
        visible: selectionBorderCanvas.visible
        radius: 8
        samples: 17
        color: Qt.rgba(1, 1, 1, 0.6)
        opacity: 0.35
        spread: 0
        transparentBorder: false
        cached: false
    }

    Canvas {
        id: selectionBorderCanvas
        anchors.fill: parent
        renderStrategy: Canvas.Threaded
        visible: root.isDrawing || root.hasSelection

        onPaint: {
            var ctx = getContext("2d")
            ctx.reset()

            if (root.selW <= 0 || root.selH <= 0) return

            ctx.lineWidth = root.isDrawing ? 1.5 : 2

            if (root.isDrawing) {
                ctx.strokeStyle = Qt.rgba(1, 1, 1, 0.92)
            } else {
                var centerX = root.selX + root.selW / 2
                var centerY = root.selY + root.selH / 2
                var maxDim = Math.max(root.selW, root.selH)
                var gradient = ctx.createRadialGradient(
                    centerX, centerY, 0,
                    centerX, centerY, maxDim * 0.8
                )
                gradient.addColorStop(0.0, Qt.rgba(1, 1, 1, 0.95))
                gradient.addColorStop(0.5, Qt.rgba(0.9, 0.9, 0.9, 0.8))
                gradient.addColorStop(1.0, Qt.rgba(0.7, 0.7, 0.7, 0.6))
                ctx.strokeStyle = gradient
            }

            root.drawSelectionPath(ctx, root.selX, root.selY, root.selW, root.selH)
            ctx.stroke()
        }
    }

    MouseArea {
        id: mouseArea
        anchors.fill: parent
        hoverEnabled: true
        preventStealing: true
        cursorShape: Qt.CrossCursor

        onPressed: function(mouse) {
            var start = Qt.point(mouse.x, mouse.y)
            root.startPoint = start
            root.endPoint = start
            root.pendingEndPoint = start
            root.hasPendingEndPoint = false
            root.isDrawing = true
            root.hasSelection = false
            root.queueRepaint()
            root.controller.beginSelection()
        }

        onPositionChanged: function(mouse) {
            if (root.isDrawing && (mouse.buttons & Qt.LeftButton)) {
                root.setPendingEndPoint(mouse.x, mouse.y, false)
            }
        }

        onReleased: function(mouse) {
            if (root.isDrawing) {
                var finalPoint = Qt.point(mouse.x, mouse.y)
                root.setPendingEndPoint(finalPoint.x, finalPoint.y, true)
                root.isDrawing = false
                root.hasSelection = true
                root.queueRepaint()
                root.controller.finishTraditionalCapture(root.startPoint, finalPoint)
            }
        }

        onCanceled: {
            root.isDrawing = false
            root.hasSelection = false
            root.hasPendingEndPoint = false
            root.queueRepaint()
        }
    }

    Component.onCompleted: {
        root.queueRepaint()
    }

}
