// Touchpad gestures for the `Gestures` bridge: an event filter on the window.
#pragma once

#include <QtCore/QObject>

namespace ion {

// Watch `window` (the QQuickWindow) for touchpad scrolling and pinching and
// report it to `target` (the Gestures bridge) through its invokables:
//
//   bool scroll(double dx, double dy, int phase, double x, double y)
//   void pause()        no phased scroll events for a moment
//   bool pinch(int stage, double delta, double x, double y)
//                       stage 0 begins, 1 moves, 2 ends
//
// `phase` is Qt::ScrollPhase; positions are in window coordinates. An
// invokable that returns true swallows the event. The filter is owned by
// `window`. Returns false if either object is null.
bool installGestureFilter(QObject *window, QObject *target);

} // namespace ion
