// System media controls on Linux: Ion as an MPRIS player on the session bus,
// so media keys, the desktop's media widget, playerctl and lock screens can
// see and control what a tab plays. Does nothing on other systems.
#pragma once

#include <QtCore/QString>
#include <QtCore/QtTypes>

#include <cstdint>

namespace ion {

struct MprisTrack
{
    QString title;
    QString artist;
    QString album;
    QString artUrl;
    QString url;
    qint64 lengthUs = 0;
    qint64 positionUs = 0;
    bool playing = false;
    bool canPrevious = false;
    bool canNext = false;
};

// Show `title`… on the bus as what Ion plays now, or, with `active` false,
// leave the bus (nothing is playing). Actions people take there come back
// through `mprisAction` (bridge/media.rs).
void mprisUpdate(bool active, const QString &title, const QString &artist, const QString &album,
                 const QString &artUrl, const QString &url, std::int64_t lengthUs, std::int64_t positionUs,
                 bool playing, bool canPrevious, bool canNext);

} // namespace ion
