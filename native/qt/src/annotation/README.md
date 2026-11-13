# Capture annotations

The stroke smoother, blur passes, drawing tools, counters, text rendering, and dock icons are forked from [paint-qt](https://github.com/a7mddra/paint-qt/tree/d53343e4f5ee06890c5bdb39e3800ab696280447/capture/native).

Snapshot and Ask Google each own an `AnnotationDocument` containing the complete frozen display in physical pixels. The shared editor maps input through the display pixel ratio. Crop changes never change annotation coordinates. Copy, export, and Send flatten the document before cropping; plain output copies the original capture.

All annotation objects use the same affine transform and local erasure mask. The document owns object selection, group transforms, endpoint and radius editing, clipboard objects, text drafts, and undo history. Brush smoothing uses the fork's streamline filter at 12% while drawing and its streamline and curve fitting pipeline at 50% on release. Blur and highlighter paths use raw points.

Object selection and movement belong to Select. Shapes and text show transformation handles immediately and let their creation tools resize, rotate, or adjust the current object. Hover and pointer events use a Qt Quick MouseArea, including the same standard resize cursors as the crop handles, the standard four-arrow rotation cursor, and a hidden system pointer for brush-style tools. Counter numbers remain contiguous after deletion or cutting, and pasted counters are appended with new numbers. The dock always shows the next available counter number.
