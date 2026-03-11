#include "PlatformIntegrationPage.h"
#include "../DeployWizard.h"

#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QLabel>
#include <QPushButton>
#include <QGroupBox>
#include <QScrollArea>
#include <QFrame>
#include <QDesktopServices>
#include <QTimer>
#include <QUrl>

PlatformIntegrationPage::PlatformIntegrationPage(QWidget *parent)
    : QWizardPage(parent)
{
    setTitle(tr("平台集成配置（可选）"));
    setSubTitle(tr("配置国内主流办公平台的机器人通知，选择需要的平台并按提示完成 Webhook 配置。"));

    // 使用滚动区域，确保窄屏可用
    auto *outerLayout = new QVBoxLayout(this);
    outerLayout->setContentsMargins(0, 0, 0, 0);

    auto *scroll = new QScrollArea(this);
    scroll->setWidgetResizable(true);
    scroll->setFrameShape(QFrame::NoFrame);

    auto *content = new QWidget(scroll);
    auto *vbox = new QVBoxLayout(content);
    vbox->setSpacing(10);

    // 企业微信
    addPlatformRow(content, vbox,
        "💬", tr("企业微信 Work"),
        tr("向关注的成员/群聊发送通知消息（群机器人 Webhook）"),
        "https://developer.work.weixin.qq.com/document/path/91770",
        ConfigKey::EnableWx, ConfigKey::WxWebhook, m_wx);

    // QQ Work
    addPlatformRow(content, vbox,
        "🐧", tr("QQ Work（腾讯企业QQ）"),
        tr("通过 QQ 工作台机器人推送消息到频道"),
        "https://work.qq.com/",
        ConfigKey::EnableQq, ConfigKey::QqWebhook, m_qq);

    // 钉钉
    addPlatformRow(content, vbox,
        "🔔", tr("钉钉（DingTalk）"),
        tr("通过钉钉自定义机器人发送群消息"),
        "https://open.dingtalk.com/document/robots/custom-robot-access",
        ConfigKey::EnableDt, ConfigKey::DtWebhook, m_dt);

    // 飞书
    addPlatformRow(content, vbox,
        "🪶", tr("飞书（Feishu / Lark）"),
        tr("通过飞书自定义机器人推送消息"),
        "https://open.feishu.cn/document/client-docs/bot-v3/add-custom-bot",
        ConfigKey::EnableFs, ConfigKey::FsWebhook, m_fs);

    vbox->addStretch();
    scroll->setWidget(content);
    outerLayout->addWidget(scroll);
}

void PlatformIntegrationPage::addPlatformRow(
        QWidget *parent, QLayout *layout,
        const QString &iconText,
        const QString &name,
        const QString &desc,
        const QString &guideUrl,
        const QString &fieldEnable,
        const QString &fieldWebhook,
        PlatformEntry &entry)
{
    auto *group = new QGroupBox(parent);
    group->setCheckable(false);

    auto *vbox = new QVBoxLayout(group);
    vbox->setSpacing(6);

    // 标题行
    auto *titleRow = new QHBoxLayout;
    auto *ico = new QLabel(iconText, group);
    ico->setFixedWidth(28);
    QFont f = ico->font();
    f.setPixelSize(18);
    ico->setFont(f);

    entry.enableBox = new QCheckBox(name, group);
    QFont bf = entry.enableBox->font();
    bf.setBold(true);
    entry.enableBox->setFont(bf);

    titleRow->addWidget(ico);
    titleRow->addWidget(entry.enableBox);
    titleRow->addStretch();
    vbox->addLayout(titleRow);

    // 描述
    auto *descLbl = new QLabel(desc, group);
    descLbl->setStyleSheet("color:#555; padding-left:34px;");
    descLbl->setWordWrap(true);
    vbox->addWidget(descLbl);

    // Webhook 输入 + 打开官方页面按钮
    auto *inputRow = new QHBoxLayout;
    inputRow->setContentsMargins(34, 0, 0, 0);

    entry.webhookEdit = new QLineEdit(group);
    entry.webhookEdit->setPlaceholderText("https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=...");
    entry.webhookEdit->setEnabled(false);

    auto *openBtn = new QPushButton(tr("前往官方配置页 ↗"), group);
    openBtn->setEnabled(false);
    openBtn->setFixedWidth(150);
    openBtn->setObjectName("openUrlBtn");

    inputRow->addWidget(entry.webhookEdit, 1);
    inputRow->addWidget(openBtn);
    vbox->addLayout(inputRow);

    // 注册字段
    registerField(QString(fieldEnable),   entry.enableBox);
    registerField(QString(fieldWebhook),  entry.webhookEdit);

    // 勾选 checkbox 时，激活 Webhook 输入框并自动打开配置页
    connect(entry.enableBox, &QCheckBox::toggled, this,
            [entry, openBtn, guideUrl](bool checked) {
                entry.webhookEdit->setEnabled(checked);
                openBtn->setEnabled(checked);
                if (checked) {
                    // 延迟 500ms 避免操作过快
                    QTimer::singleShot(500, [guideUrl] {
                        QDesktopServices::openUrl(QUrl(guideUrl));
                    });
                }
            });

    connect(openBtn, &QPushButton::clicked, this, [guideUrl] {
        QDesktopServices::openUrl(QUrl(guideUrl));
    });

    layout->addWidget(group);
}

void PlatformIntegrationPage::initializePage()
{
    // 平台集成无需预填充，默认全部不选
}
