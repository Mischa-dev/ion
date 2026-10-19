#include "print.h"

#include <QtCore/QCoreApplication>
#include <QtGui/QImage>
#include <QtGui/QPageLayout>
#include <QtGui/QPageSize>
#include <QtGui/QPainter>
#include <QtPdf/QPdfDocument>
#include <QtPrintSupport/QPrinter>
#include <QtPrintSupport/QPrinterInfo>

namespace ion {

QStringList printerNames()
{
    return QPrinterInfo::availablePrinterNames();
}

QString defaultPrinterName()
{
    return QPrinterInfo::defaultPrinterName();
}

QString printPdf(const QString &pdfPath, const QString &printerName, const QString &title,
                 const QList<qint32> &pages, qint32 copies, bool color, bool duplex)
{
    QPdfDocument document;
    if (document.load(pdfPath) != QPdfDocument::Error::None)
        return QCoreApplication::translate("Printing", "The preview could not be read.");
    const QPrinterInfo info = QPrinterInfo::printerInfo(printerName);
    if (info.isNull())
        return QCoreApplication::translate("Printing", "%1 is not available.").arg(printerName);
    if (pages.isEmpty())
        return QCoreApplication::translate("Printing", "No pages to print.");

    QPrinter printer(info, QPrinter::HighResolution);
    printer.setDocName(title);
    printer.setCopyCount(qMax(1, copies));
    printer.setColorMode(color ? QPrinter::Color : QPrinter::GrayScale);
    printer.setDuplex(duplex ? QPrinter::DuplexAuto : QPrinter::DuplexNone);
    printer.setFullPage(true);

    // The engine already laid the page out on the chosen paper, margins
    // included; the printer gets the same paper and orientation.
    const QSizeF first = document.pagePointSize(pages.first());
    const bool landscape = first.width() > first.height();
    const QSizeF portrait(qMin(first.width(), first.height()), qMax(first.width(), first.height()));
    printer.setPageLayout(QPageLayout(QPageSize(portrait, QPageSize::Point),
                                      landscape ? QPageLayout::Landscape : QPageLayout::Portrait,
                                      QMarginsF()));

    QPainter painter;
    if (!painter.begin(&printer))
        return QCoreApplication::translate("Printing", "%1 did not accept the job.").arg(printerName);
    const qreal dpi = qMin<qreal>(printer.resolution(), 300);
    bool firstPage = true;
    for (const qint32 page : pages) {
        if (page < 0 || page >= document.pageCount())
            continue;
        if (!firstPage && !printer.newPage())
            break;
        firstPage = false;
        const QSizeF points = document.pagePointSize(page);
        const QImage image = document.render(page, (points * dpi / 72.0).toSize());
        const QRectF area = painter.viewport();
        const QSizeF fitted = points.scaled(area.size(), Qt::KeepAspectRatio);
        const QRectF target(area.x() + (area.width() - fitted.width()) / 2,
                            area.y() + (area.height() - fitted.height()) / 2,
                            fitted.width(), fitted.height());
        painter.drawImage(target, image);
    }
    painter.end();
    return {};
}

} // namespace ion
