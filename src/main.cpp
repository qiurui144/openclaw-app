#include "DeployWizard.h"

#include <QApplication>
#include <QScreen>
#include <QLocale>
#include <QTranslator>
#include <QStyleFactory>

int main(int argc, char *argv[])
{
    QApplication app(argc, argv);

    // 应用信息
    app.setApplicationName("openclaw");
    app.setApplicationDisplayName("OpenClaw 部署向导");
    app.setApplicationVersion("1.0.0");
    app.setOrganizationName("OpenClaw");
    app.setOrganizationDomain("openclaw.io");

    // 高DPI支持（Qt5/Qt6通用）
#if QT_VERSION < QT_VERSION_CHECK(6, 0, 0)
    QApplication::setAttribute(Qt::AA_EnableHighDpiScaling);
    QApplication::setAttribute(Qt::AA_UseHighDpiPixmaps);
#endif

    // 使用 Fusion 风格保证跨平台一致性
    app.setStyle(QStyleFactory::create("Fusion"));

    DeployWizard wizard;

    // 居中显示
    QScreen *screen = wizard.screen();
    if (screen) {
        QRect geo = screen->availableGeometry();
        wizard.move(geo.center() - wizard.rect().center());
    }

    wizard.show();
    return app.exec();
}
