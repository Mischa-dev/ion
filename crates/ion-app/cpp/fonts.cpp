#include "fonts.h"

#include <QtCore/QDir>
#include <QtCore/QStringList>
#include <QtGui/QFont>
#include <QtGui/QFontDatabase>
#include <QtGui/QGuiApplication>

namespace ion {

void installUiFont()
{
    const QString dir = qEnvironmentVariable("ION_FONT_DIR");
    if (!dir.isEmpty()) {
        QDir fonts(dir);
        const QStringList files = fonts.entryList({ QStringLiteral("*.ttf"), QStringLiteral("*.otf") },
                                                  QDir::Files);
        for (const QString &file : files)
            QFontDatabase::addApplicationFont(fonts.filePath(file));
    }
#ifndef Q_OS_MACOS
    // The first of these that is installed; otherwise the platform default.
    const QStringList preferred = { QStringLiteral("Inter Variable"), QStringLiteral("Inter"),
                                    QStringLiteral("Inter Display") };
    const QStringList families = QFontDatabase::families();
    for (const QString &family : preferred) {
        if (families.contains(family)) {
            QFont font = QGuiApplication::font();
            font.setFamilies({ family });
            font.setHintingPreference(QFont::PreferNoHinting);
            QGuiApplication::setFont(font);
            break;
        }
    }
#endif
}

} // namespace ion
