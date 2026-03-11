#pragma once
#include <QWizard>
#include <QMap>
#include <QString>

class DeployEngine;

enum PageId {
    Page_Welcome               = 0,
    Page_SystemCheck           = 1,
    Page_InstallConfig         = 2,
    Page_ServiceConfig         = 3,
    Page_PlatformIntegration   = 4,
    Page_Deployment            = 5,
    Page_Finish                = 6
};

// 共享配置键名
namespace ConfigKey {
    inline constexpr auto InstallPath    = "installPath";
    inline constexpr auto ServicePort    = "servicePort";
    inline constexpr auto AdminPassword  = "adminPassword";
    inline constexpr auto DomainName     = "domainName";
    inline constexpr auto WxWebhook      = "wxWebhook";
    inline constexpr auto QqWebhook      = "qqWebhook";
    inline constexpr auto DtWebhook      = "dtWebhook";
    inline constexpr auto FsWebhook      = "fsWebhook";
    inline constexpr auto EnableWx       = "enableWx";
    inline constexpr auto EnableQq       = "enableQq";
    inline constexpr auto EnableDt       = "enableDt";
    inline constexpr auto EnableFs       = "enableFs";
    inline constexpr auto InstallService = "installService";
    inline constexpr auto StartOnBoot    = "startOnBoot";
}

class DeployWizard : public QWizard {
    Q_OBJECT
public:
    explicit DeployWizard(QWidget *parent = nullptr);
    ~DeployWizard() override = default;

    DeployEngine *deployEngine() const { return m_engine; }

private:
    void setupStyle();
    void setupPages();
    static QPixmap makeBannerPixmap();
    static QPixmap makeLogoPixmap();

    DeployEngine *m_engine;
};
