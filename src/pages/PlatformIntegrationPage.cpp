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
#include <QRegularExpression>
#include <QToolTip>
#include <QMessageBox>

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

    // Webhook 输入 + 校验状态 + 打开官方页面按钮
    auto *inputRow = new QHBoxLayout;
    inputRow->setContentsMargins(34, 0, 0, 0);

    entry.webhookEdit = new QLineEdit(group);
    entry.webhookEdit->setPlaceholderText("https://");
    entry.webhookEdit->setEnabled(false);

    // 实时输入校验状态标签
    auto *validLbl = new QLabel(group);
    validLbl->setFixedWidth(20);
    validLbl->setAlignment(Qt::AlignCenter);
    validLbl->setToolTip(tr("Webhook URL 格式校验"));

    auto *openBtn = new QPushButton(tr("前往官方配置页 ↗"), group);
    openBtn->setEnabled(false);
    openBtn->setFixedWidth(150);
    openBtn->setObjectName("openUrlBtn");

    inputRow->addWidget(entry.webhookEdit, 1);
    inputRow->addWidget(validLbl);
    inputRow->addWidget(openBtn);
    vbox->addLayout(inputRow);

    // 注册字段
    registerField(QString(fieldEnable),   entry.enableBox);
    registerField(QString(fieldWebhook),  entry.webhookEdit);

    // 实时校验 Webhook URL（必须以 https:// 开头，含 query 参数）
    static const QRegularExpression urlRe(
        R"(^https://[a-zA-Z0-9\-\.]+(/[^\s]*)+\?[^\s]+$)");

    auto validateWebhook = [entry, validLbl, urlRe]() {
        const QString url = entry.webhookEdit->text().trimmed();
        if (url.isEmpty()) {
            validLbl->setText("");
            entry.webhookEdit->setStyleSheet("");
        } else if (urlRe.match(url).hasMatch()) {
            validLbl->setText("\u2713");
            validLbl->setStyleSheet("color:#2e7d32; font-weight:bold;");
            entry.webhookEdit->setStyleSheet("border-color:#2e7d32;");
        } else {
            validLbl->setText("\u26a0");
            validLbl->setStyleSheet("color:#e65100; font-weight:bold;");
            entry.webhookEdit->setStyleSheet("border-color:#e65100;");
        }
    };

    connect(entry.webhookEdit, &QLineEdit::textChanged, this,
            [validateWebhook]() { validateWebhook(); });

    // 勾选 checkbox 时，激活 Webhook 输入框并自动打开配置页
    connect(entry.enableBox, &QCheckBox::toggled, this,
            [entry, openBtn, validLbl, guideUrl, validateWebhook](bool checked) {
                entry.webhookEdit->setEnabled(checked);
                openBtn->setEnabled(checked);
                if (!checked) {
                    validLbl->setText("");
                    entry.webhookEdit->setStyleSheet("");
                } else {
                    validateWebhook();
                    // 延迟 600ms 打开官方配置页，避免刚勾选就弹出太突兀
                    QTimer::singleShot(600, [guideUrl] {
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

bool PlatformIntegrationPage::validateEntry(
        const PlatformEntry &entry, const QString &platformName) const
{
    if (!entry.enableBox || !entry.enableBox->isChecked())
        return true;  // 未启用，直接通过

    const QString url = entry.webhookEdit ? entry.webhookEdit->text().trimmed() : QString();
    if (url.isEmpty()) {
        QMessageBox::warning(const_cast<PlatformIntegrationPage *>(this),
            tr("配置不完整"),
            tr("%1 已启用，但 Webhook URL 为空。\n请填写 URL 或取消勾选该平台。")
                .arg(platformName));
        return false;
    }

    static const QRegularExpression urlRe(
        R"(^https://[a-zA-Z0-9\-\.]+(/[^\s]*)+\?[^\s]+$)");
    if (!urlRe.match(url).hasMatch()) {
        QMessageBox::warning(const_cast<PlatformIntegrationPage *>(this),
            tr("URL 格式错误"),
            tr("%1 的 Webhook URL 格式不正确。\n"
               "URL 应以 https:// 开头并包含必要的查询参数（如 ?key=...）。")
                .arg(platformName));
        entry.webhookEdit->setFocus();
        return false;
    }
    return true;
}

bool PlatformIntegrationPage::validatePage()
{
    if (!validateEntry(m_wx, tr("企业微信")))  return false;
    if (!validateEntry(m_qq, tr("QQ Work")))   return false;
    if (!validateEntry(m_dt, tr("钉钉")))       return false;
    if (!validateEntry(m_fs, tr("飞书")))       return false;
    return true;
}
