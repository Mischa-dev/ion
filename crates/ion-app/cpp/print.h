// Printing for the `Printing` bridge: the printers Qt knows and sending a
// PDF (the engine's print of the page) to one of them, page by page.
#pragma once

#include <QtCore/QList>
#include <QtCore/QString>
#include <QtCore/QStringList>

namespace ion {

// Names of the printers the system offers, and the default one ("" if none).
QStringList printerNames();
QString defaultPrinterName();

// Print `pages` (0-based) of the PDF at `pdfPath` on `printer`. Returns ""
// once the job is handed to the printer, else what went wrong. Safe to call
// off the GUI thread; it blocks while pages are rendered.
QString printPdf(const QString &pdfPath, const QString &printer, const QString &title,
                 const QList<qint32> &pages, qint32 copies, bool color, bool duplex);

} // namespace ion
