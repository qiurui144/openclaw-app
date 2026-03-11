#include "DeployWizard.h"
#include "core/DeployEngine.h"
#include "pages/WelcomePage.h"
#include "pages/SystemCheckPage.h"
#include "pages/InstallConfigPage.h"
#include "pages/ServiceConfigPage.h"
#include "pages/PlatformIntegrationPage.h"
#include "pages/DeploymentPage.h"
#include "pages/FinishPage.h"

#include <QFile>
#include <QPainter>
#include <QLinearGradient>
#include <QFont>
#include <QFontMetrics>
#include <QApplication>

DeployWizard::DeployWizard(QWidget *parent)
    : QWizard(parent)
    , m_engine(new DeployEngine(this))
{
    setWindowTitle(tr("OpenClaw 一键部署向导"));
    setWizardStyle(QWizard::ModernStyle);
    setWindowIcon(QIcon(":/icons/app.svg"));
    setFixedSize(860, 620);
    setPixmap(QWizard::BannerPixmap, makeBannerPixmap());
    setPixmap(QWizard::LogoPixmap, makeLogoPixmap());

    setOption(QWizard::HaveHelpButton, false);
    setOption(QWizard::NoCancelButtonOnLastPage, true);
    setButtonText(QWizard::NextButton, tr("下一步 >"));
    setButtonText(QWizard::BackButton, tr("< 上一步"));
    setButtonText(QWizard::FinishButton, tr("完成并启动"));
    setButtonText(QWizard::CancelButton, tr("取消"));

    setupPages();
    setupStyle();
}

void DeployWizard::setupPages()
{
    setPage(Page_Welcome,             new WelcomePage(this));
    setPage(Page_SystemCheck,         new SystemCheckPage(this));
    setPage(Page_InstallConfig,       new InstallConfigPage(this));
    setPage(Page_ServiceConfig,       new ServiceConfigPage(this));
    setPage(Page_PlatformIntegration, new PlatformIntegrationPage(this));
    setPage(Page_Deployment,          new DeploymentPage(this));
    setPage(Page_Finish,              new FinishPage(this));
    setStartId(Page_Welcome);
}

void DeployWizard::setupStyle()
{
    QFile styleFile(":/styles/wizard.qss");
    if (styleFile.open(QFile::ReadOnly)) {
        setStyleSheet(QString::fromUtf8(styleFile.readAll()));
        styleFile.close();
    }
}

QPixmap DeployWizard::makeBannerPixmap()
{
    QPixmap px(340, 80);
    px.fill(Qt::transparent);

    QPainter p(&px);
    p.setRenderHint(QPainter::Antialiasing);

    QLinearGradient grad(0, 0, 340, 80);
    grad.setColorAt(0.0, QColor(0x0D, 0x47, 0xA1));
    grad.setColorAt(1.0, QColor(0x19, 0x76, 0xD2));
    p.fillRect(px.rect(), grad);

    p.setPen(Qt::white);
    QFont f = QApplication::font();
    f.setPixelSize(22);
    f.setBold(true);
    p.setFont(f);
    p.drawText(QRect(16, 8, 310, 36), Qt::AlignVCenter | Qt::AlignLeft, "OpenClaw");

    f.setPixelSize(12);
    f.setBold(false);
    p.setFont(f);
    p.setPen(QColor(0xBB, 0xDE, 0xFB));
    p.drawText(QRect(16, 44, 310, 28), Qt::AlignVCenter | Qt::AlignLeft,
               tr("企业级一键部署 · 国内网络优化"));
    return px;
}

QPixmap DeployWizard::makeLogoPixmap()
{
    QPixmap px(48, 48);
    px.fill(Qt::transparent);

    QPainter p(&px);
    p.setRenderHint(QPainter::Antialiasing);

    p.setBrush(QColor(0x0D, 0x47, 0xA1));
    p.setPen(Qt::NoPen);
    p.drawRoundedRect(px.rect(), 10, 10);

    p.setPen(Qt::white);
    QFont f = QApplication::font();
    f.setPixelSize(24);
    f.setBold(true);
    p.setFont(f);
    p.drawText(px.rect(), Qt::AlignCenter, "OC");
    return px;
}
