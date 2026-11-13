# Capture annotations

The stroke smoother, blur passes, drawing tools, counters, text rendering, and dock icons are forked from [paint-qt](https://github.com/a7mddra/paint-qt/tree/d53343e4f5ee06890c5bdb39e3800ab696280447/capture/native).

Snapshot and Ask Google each own an `AnnotationDocument` containing the complete frozen display in physical pixels. The shared editor maps input through the display pixel ratio. Crop changes never change annotation coordinates. Copy, export, Save, and Send flatten the document before cropping; plain output copies the original capture.

All annotation objects use the same affine transform and local erasure mask. The document owns object selection, group transforms, endpoint and radius editing, clipboard objects, text drafts, and undo history. SVG insertions retain their source for vector rendering. Brush smoothing uses the fork's streamline filter at 12% while drawing and its streamline and curve fitting pipeline at 50% on release. Blur and highlighter paths use raw points.

Object selection, movement, resizing, endpoint editing, and rotation belong to Select. Drawing tools do not start object transforms. Capture cursor ownership is scoped to the hovered overlay and restored when the pointer leaves, the window loses activation, or a native dialog opens. The cursor is resolved after Qt Quick dispatch so crop handles, text editors, and annotation tools share the same native cursor behavior, including XWayland.
