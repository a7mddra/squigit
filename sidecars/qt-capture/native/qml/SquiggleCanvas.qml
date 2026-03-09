// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

import QtQuick
import Qt5Compat.GraphicalEffects

/**
 * Freehand drawing canvas with smooth strokes and GPU-accelerated glow.
 * 
 * This is the "Circle to Search" style squiggle selection mode.
 * Uses quadratic bezier curves for smooth paths and Glow for the effect.
 */

Item {
    id: root
    anchors.fill: parent
    focus: true
    
    property var controller
    
    property var strokes: []
    property bool isDrawing: false
    property point lastPoint: Qt.point(0, 0)
    
    readonly property real smoothingFactor: 0.3
    readonly property real brushSize: 7
    
    Canvas {
        id: canvas
        anchors.fill: parent
        renderStrategy: Canvas.Immediate
        
        onPaint: {
            var ctx = getContext("2d")
            ctx.clearRect(0, 0, width, height)
            
            if (root.strokes.length < 2) return
            
            ctx.strokeStyle = "white"
            ctx.lineWidth = root.brushSize
            ctx.lineCap = "round"
            ctx.lineJoin = "round"
            
            ctx.beginPath()
            ctx.moveTo(root.strokes[0].x, root.strokes[0].y)
            
            for (var i = 1; i < root.strokes.length - 1; i++) {
                var xMid = (root.strokes[i].x + root.strokes[i + 1].x) / 2
                var yMid = (root.strokes[i].y + root.strokes[i + 1].y) / 2
                ctx.quadraticCurveTo(root.strokes[i].x, root.strokes[i].y, xMid, yMid)
            }
            
            var last = root.strokes[root.strokes.length - 1]
            ctx.lineTo(last.x, last.y)
            
            ctx.stroke()
        }
    }
    
    Glow {
        anchors.fill: canvas
        source: canvas
        radius: 12
        samples: 25
        color: Qt.rgba(1, 1, 1, 0.5)
        spread: 0.2
        cached: false
        transparentBorder: true
    }
    
    MouseArea {
        id: mouseArea
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.CrossCursor
        
        onPressed: function(mouse) {
            root.strokes = []
            root.isDrawing = true
            root.lastPoint = Qt.point(mouse.x, mouse.y)
            root.strokes.push({x: mouse.x, y: mouse.y})
            canvas.requestPaint()
        }
        
        onPositionChanged: function(mouse) {
            if (!root.isDrawing) return
            
            var smoothedX = root.lastPoint.x + (mouse.x - root.lastPoint.x) * root.smoothingFactor
            var smoothedY = root.lastPoint.y + (mouse.y - root.lastPoint.y) * root.smoothingFactor
            
            root.lastPoint = Qt.point(smoothedX, smoothedY)
            root.strokes.push({x: smoothedX, y: smoothedY})
            canvas.requestPaint()
        }
        
        onReleased: function(mouse) {
            if (!root.isDrawing) return
            
            root.isDrawing = false
            
            var pointsList = []
            for (var i = 0; i < root.strokes.length; i++) {
                pointsList.push(Qt.point(root.strokes[i].x, root.strokes[i].y))
            }
            
            root.controller.finishSquiggleCapture(pointsList)
        }
    }
    
    Keys.onPressed: function(event) {
        if (event.key === Qt.Key_Escape || event.key === Qt.Key_Q) {
            root.controller.cancel()
            event.accepted = true
        }
    }
}
