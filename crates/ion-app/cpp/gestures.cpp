#include "ion-app/cpp/gestures.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QPointer>
#include <QtCore/QTimer>
#include <QtGui/QMouseEvent>
#include <QtGui/QNativeGestureEvent>
#include <QtGui/QPointingDevice>
#include <QtGui/QWheelEvent>
#include <QtQuick/QQuickItem>
#include <QtQuick/QQuickWindow>

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
        , m_window(qobject_cast<QQuickWindow *>(window))
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
        case QEvent::MouseButtonPress:
        case QEvent::MouseMove:
        case QEvent::MouseButtonRelease:
            return mouse(watched, static_cast<QMouseEvent *>(event));
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
            m_overPopup = overPopup(event->position());
            break;
        case Qt::ZoomNativeGesture:
            // A pinch that starts over a popup (downloads, menus) leaves the
            // page under it alone.
            if (m_overPopup)
                return false;
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

    bool mouse(QObject *window, QMouseEvent *event)
    {
        if (m_replaying)
            return false;
        const QPointF position = event->position();
        const double x = position.x(), y = position.y();
        switch (event->type()) {
        case QEvent::MouseButtonPress: {
            if (event->button() != Qt::RightButton || event->buttons() != Qt::RightButton
                || overPopup(position))
                return false;
            bool take = false;
            QMetaObject::invokeMethod(m_target, "strokeBegin", Qt::DirectConnection,
                                      qReturnArg(take), x, y);
            m_stroking = take;
            if (take) {
                m_pressPosition = position;
                m_pressGlobal = event->globalPosition();
                m_pressModifiers = event->modifiers();
                m_pressDevice = event->pointingDevice();
            }
            return take;
        }
        case QEvent::MouseMove:
            // The page never saw the press, so it doesn't see the drag either.
            if (!m_stroking)
                return false;
            QMetaObject::invokeMethod(m_target, "strokeTo", Qt::DirectConnection, x, y);
            return true;
        case QEvent::MouseButtonRelease: {
            if (!m_stroking || event->button() != Qt::RightButton)
                return false;
            m_stroking = false;
            bool done = false;
            QMetaObject::invokeMethod(m_target, "strokeEnd", Qt::DirectConnection,
                                      qReturnArg(done), x, y);
            if (!done)
                replayClick(window);
            return true;
        }
        default:
            return false;
        }
    }

    // A right click that wasn't a stroke: hand the page its press and release.
    void replayClick(QObject *window)
    {
        m_replaying = true;
        QMouseEvent press(QEvent::MouseButtonPress, m_pressPosition, m_pressPosition, m_pressGlobal,
                          Qt::RightButton, Qt::RightButton, m_pressModifiers, device());
        QCoreApplication::sendEvent(window, &press);
        QMouseEvent release(QEvent::MouseButtonRelease, m_pressPosition, m_pressPosition,
                            m_pressGlobal, Qt::RightButton, Qt::NoButton, m_pressModifiers,
                            device());
        QCoreApplication::sendEvent(window, &release);
        m_replaying = false;
    }

    const QPointingDevice *device() const
    {
        return m_pressDevice ? m_pressDevice : QPointingDevice::primaryPointingDevice();
    }

    // Whether a popup in the window's overlay covers `position`.
    bool overPopup(const QPointF &position) const
    {
        if (!m_window)
            return false;
        for (QQuickItem *item : m_window->contentItem()->childItems()) {
            if (!item->inherits("QQuickOverlay"))
                continue;
            const QPointF local = item->mapFromScene(position);
            if (item->isVisible() && item->childAt(local.x(), local.y()))
                return true;
        }
        return false;
    }

    QPointer<QQuickWindow> m_window;
    QPointer<QObject> m_target;
    QTimer m_pause;
    bool m_overPopup = false;
    bool m_stroking = false;
    bool m_replaying = false;
    QPointF m_pressPosition;
    QPointF m_pressGlobal;
    Qt::KeyboardModifiers m_pressModifiers;
    const QPointingDevice *m_pressDevice = nullptr;
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
