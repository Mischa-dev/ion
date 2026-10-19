#include "ion-app/cpp/gestures.h"

#include <QtCore/QPointer>
#include <QtCore/QTimer>
#include <QtGui/QNativeGestureEvent>
#include <QtGui/QWheelEvent>

namespace ion {

namespace {

// Platforms that report no scroll phases (some X11 drivers) end a gesture
// with a pause this long.
constexpr int pauseMs = 180;

class GestureFilter : public QObject
{
public:
    GestureFilter(QObject *window, QObject *target)
        : QObject(window)
        , m_target(target)
    {
        m_pause.setSingleShot(true);
        m_pause.setInterval(pauseMs);
        connect(&m_pause, &QTimer::timeout, this, [this] {
            if (m_target)
                QMetaObject::invokeMethod(m_target, "pause", Qt::DirectConnection);
        });
    }

protected:
    bool eventFilter(QObject *watched, QEvent *event) override
    {
        if (!m_target)
            return false;
        switch (event->type()) {
        case QEvent::Wheel:
            return wheel(static_cast<QWheelEvent *>(event));
        case QEvent::NativeGesture:
            return gesture(static_cast<QNativeGestureEvent *>(event));
        default:
            return QObject::eventFilter(watched, event);
        }
    }

private:
    bool wheel(QWheelEvent *event)
    {
        // Touchpads send pixel deltas or scroll phases; a mouse wheel (and
        // Shift+wheel's sideways scrolling) sends neither, and never swipes.
        // Ctrl+scroll is zoom.
        const QPointF pixels = event->pixelDelta();
        const bool phased = event->phase() != Qt::NoScrollPhase;
        if ((!phased && pixels.isNull()) || (event->modifiers() & Qt::ControlModifier))
            return false;
        if (!phased)
            m_pause.start();
        const QPointF position = event->position();
        bool swallow = false;
        QMetaObject::invokeMethod(m_target, "scroll", Qt::DirectConnection,
                                  qReturnArg(swallow),
                                  double(pixels.x()), double(pixels.y()),
                                  int(event->phase()),
                                  double(position.x()), double(position.y()));
        return swallow;
    }

    bool gesture(QNativeGestureEvent *event)
    {
        int stage = -1;
        double delta = 0;
        switch (event->gestureType()) {
        case Qt::BeginNativeGesture:
            stage = 0;
            break;
        case Qt::ZoomNativeGesture:
            stage = 1;
            delta = event->value();
            break;
        case Qt::EndNativeGesture:
            stage = 2;
            break;
        default:
            return false;
        }
        const QPointF position = event->position();
        bool swallow = false;
        QMetaObject::invokeMethod(m_target, "pinch", Qt::DirectConnection,
                                  qReturnArg(swallow), stage, delta,
                                  double(position.x()), double(position.y()));
        return swallow;
    }

    QPointer<QObject> m_target;
    QTimer m_pause;
};

} // namespace

bool installGestureFilter(QObject *window, QObject *target)
{
    if (!window || !target)
        return false;
    window->installEventFilter(new GestureFilter(window, target));
    return true;
}

} // namespace ion
