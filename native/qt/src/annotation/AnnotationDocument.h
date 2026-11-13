#pragma once

#include <QColor>
#include <QHash>
#include <QImage>
#include <QObject>
#include <QPainterPath>
#include <QPolygonF>
#include <QSet>
#include <QTimer>
#include <QTransform>
#include <QVariantMap>
#include <QVector>

class QPainter;

struct Annotation {
  QString tool;
  QPainterPath path;
  QPainterPath erased;
  QPainterPath fragment;
  qreal fragmentSize = 0;
  QColor color;
  qreal width = 4;
  qreal opacity = 1;
  QRectF rect;
  QString text;
  int number = 0;
  quint64 counterId = 0;
  qreal radius = 0;
  bool autoHeight = true;
  QPointF start;
  QPointF end;
  QImage blur;
  QRectF blurRect;
  QTransform transform;
};

class AnnotationDocument : public QObject {
  Q_OBJECT
  Q_PROPERTY(bool canUndo READ canUndo NOTIFY historyChanged)
  Q_PROPERTY(bool canRedo READ canRedo NOTIFY historyChanged)
  Q_PROPERTY(bool hasSelection READ hasSelection NOTIFY selectionChanged)
  Q_PROPERTY(bool canTransformSelection READ canTransformSelection NOTIFY selectionChanged)
  Q_PROPERTY(int nextCounter READ nextCounter NOTIFY changed)
  Q_PROPERTY(int selectionCount READ selectionCount NOTIFY selectionChanged)
  Q_PROPERTY(bool hasTextSelection READ hasTextSelection NOTIFY selectionChanged)
  Q_PROPERTY(bool editingText READ editingText NOTIFY textBoxChanged)
  Q_PROPERTY(QVariantMap textBox READ textBox NOTIFY textBoxChanged)
  Q_PROPERTY(QString tool READ tool NOTIFY styleChanged)
  Q_PROPERTY(QColor inkColor READ inkColor WRITE setInkColor NOTIFY styleChanged)
  Q_PROPERTY(qreal toolSize READ toolSize WRITE setToolSize NOTIFY styleChanged)
  Q_PROPERTY(bool solidTool READ solidTool NOTIFY styleChanged)
  Q_PROPERTY(bool colorTool READ colorTool NOTIFY styleChanged)
  Q_PROPERTY(bool mixedColors READ mixedColors NOTIFY selectionChanged)
  Q_PROPERTY(QString status READ status NOTIFY statusChanged)

public:
  explicit AnnotationDocument(QObject *parent = nullptr);
  void setImage(const QImage &image);
  QSize imageSize() const { return m_image.size(); }
  bool canUndo() const { return m_historyIndex > 0; }
  bool canRedo() const { return m_historyIndex + 1 < m_history.size(); }
  bool hasSelection() const { return !m_selected.isEmpty(); }
  int selectionCount() const { return m_selected.size(); }
  bool hasTextSelection() const;
  bool canTransformSelection() const;
  int nextCounter() const;
  bool editingText() const { return m_textIndex >= 0; }
  QVariantMap textBox() const;
  QString tool() const { return m_tool; }
  QColor inkColor() const { return m_color; }
  qreal toolSize() const { return m_size; }
  bool solidTool() const;
  bool colorTool() const;
  bool mixedColors() const;
  QString status() const { return m_status; }
  void setInkColor(const QColor &color);
  void setToolSize(qreal size);
  void paint(QPainter &painter, bool interactive = true) const;
  QImage flattened() const;
  void begin(QPointF point, Qt::KeyboardModifiers modifiers, qreal tolerance);
  void move(QPointF point, Qt::KeyboardModifiers modifiers);
  void finish(QPointF point, Qt::KeyboardModifiers modifiers);
  void editTextAt(QPointF point, qreal tolerance);
  QString contextAt(QPointF point, qreal tolerance);
  QPolygonF selectionPolygon() const;
  QVector<QPolygonF> selectedPolygons() const;
  QVector<QPointF> selectionHandles(qreal pixelSize) const;
  int selectionHandleAt(QPointF point, qreal tolerance) const;
  void setStatus(const QString &status);

  Q_INVOKABLE void beginFormatting();
  Q_INVOKABLE void finishFormatting();
  Q_INVOKABLE void selectTool(const QString &tool);
  Q_INVOKABLE void undo();
  Q_INVOKABLE void redo();
  Q_INVOKABLE void clear();
  Q_INVOKABLE void deleteSelected();
  Q_INVOKABLE void selectAll();
  Q_INVOKABLE void deselect();
  Q_INVOKABLE void copySelected();
  Q_INVOKABLE void cutSelected();
  Q_INVOKABLE void paste();
  Q_INVOKABLE void updateText(const QString &text);
  Q_INVOKABLE void setTextContentHeight(qreal height);
  Q_INVOKABLE void finishTextEditing(bool accept = true);
  Q_INVOKABLE void cancelStroke();
  Q_INVOKABLE bool handleEscape();

signals:
  void imageChanged();
  void historyChanged();
  void selectionChanged();
  void styleChanged();
  void textBoxChanged();
  void statusChanged();
  void changed();

private:
  enum class Gesture { None, Move, Resize, Rotate, Radius, Endpoint, CreateText, Erase };
  void commit();
  void notifySelection();
  void captureGesture();
  void transformSelection(QPointF point, Qt::KeyboardModifiers modifiers);
  void beginTextEditing(int index, bool draft = false);
  void paintAnnotation(QPainter &painter, const Annotation &item) const;
  void updateShape(Annotation &item) const;
  void updateActivePath(bool final);
  void prepareBlur();
  void cropBlur();
  void eraseAt(QPointF point);
  void splitErasedItems();
  int hitTest(QPointF point, qreal tolerance) const;
  QRectF localBounds(const Annotation &item) const;
  QRectF selectionBounds() const;
  bool lineSelection() const;
  bool radiusSelection() const;
  void renumberCounters();

  QImage m_image;
  QVector<Annotation> m_items;
  QVector<QVector<Annotation>> m_history;
  int m_historyIndex = 0;
  QSet<int> m_selected;
  QVector<Annotation> m_beforeGesture;
  QPolygonF m_gesturePolygon;
  QPointF m_start;
  QPointF m_last;
  qreal m_pixelSize = 1;
  int m_handle = -1;
  Gesture m_gesture = Gesture::None;
  bool m_drawing = false;
  bool m_erased = false;
  QVector<bool> m_erasedItems;
  bool m_formatting = false;
  bool m_formatModified = false;
  Annotation m_active;
  QVector<QPointF> m_points;
  QTimer m_strokeUpdate;
  int m_textIndex = -1;
  bool m_textDraft = false;
  Annotation m_textOriginal;
  QString m_tool = "select";
  QColor m_color = QColor("#f04438");
  qreal m_size = 5;
  QHash<QString, qreal> m_sizes;
  QString m_status;
  quint64 m_counterSerial = 0;
};
