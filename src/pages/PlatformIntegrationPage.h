#pragma once
#include <QWizardPage>
#include <QCheckBox>
#include <QLineEdit>
#include <QStackedWidget>

class PlatformIntegrationPage : public QWizardPage {
    Q_OBJECT
public:
    explicit PlatformIntegrationPage(QWidget *parent = nullptr);
    void initializePage() override;
    bool isComplete() const override { return true; }  // 平台集成为可选步骤

private:
    struct PlatformEntry {
        QCheckBox  *enableBox   = nullptr;
        QLineEdit  *webhookEdit = nullptr;
    };

    PlatformEntry m_wx;  // 企业微信
    PlatformEntry m_qq;  // QQ Work
    PlatformEntry m_dt;  // 钉钉
    PlatformEntry m_fs;  // 飞书

    void addPlatformRow(QWidget *parent, QLayout *layout,
                        const QString &iconText,
                        const QString &name,
                        const QString &desc,
                        const QString &guideUrl,
                        const QString &fieldEnable,
                        const QString &fieldWebhook,
                        PlatformEntry &entry);
};
