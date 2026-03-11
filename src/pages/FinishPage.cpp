#include "FinishPage.h"
#include "../DeployWizard.h"
#include "../core/DeployEngine.h"

#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QLabel>
#include <QPushButton>
#include <QDesktopServices>
#include <QUrl>
#include <QTimer>

FinishPage::FinishPage(QWidget *parent)
    : QWizardPage(parent)
{
    setTitle(tr("部署完成 🎉"));
    setSubTitle(tr("OpenClaw 服务已成功部署并启动，管理控制台已在浏览器中打开。"));
    setFinalPage(true);

    auto *root = new QVBoxLayout(this);
    root->setSpacing(14);

    // 成功图标
    auto *iconRow = new QHBoxLayout;
    auto *iconLbl = new QLabel("✅", this);
    QFont f = iconLbl->font();
    f.setPixelSize(48);
    iconLbl->setFont(f);
    iconLbl->setAlignment(Qt::AlignCenter);
    iconRow->addStretch();
    iconRow->addWidget(iconLbl);
    iconRow->addStretch();
    root->addLayout(iconRow);

    auto *desc = new QLabel(
        tr("<p>OpenClaw 服务已完成部署。<br>"
           "若浏览器未自动打开，请手动访问下方控制台地址。</p>"),
        this);
    desc->setWordWrap(true);
    desc->setAlignment(Qt::AlignCenter);
    root->addWidget(desc);

    // 控制台链接按钮
    auto *btnOpen = new QPushButton(tr("🌐 打开管理控制台"), this);
    btnOpen->setObjectName("primaryBtn");
    btnOpen->setFixedHeight(36);
    connect(btnOpen, &QPushButton::clicked, this, &FinishPage::openDashboard);
    root->addWidget(btnOpen);

    root->addSpacing(8);

    // 帮助链接
    auto *helpLbl = new QLabel(
        tr("遇到问题？查阅 <a href='https://github.com/openclaw/docs'>在线文档</a> "
           "或提交 <a href='https://github.com/openclaw/issues'>Issue</a>"),
        this);
    helpLbl->setOpenExternalLinks(true);
    helpLbl->setAlignment(Qt::AlignCenter);
    root->addWidget(helpLbl);

    root->addStretch();
}

void FinishPage::initializePage()
{
    // 进入完成页后自动打开控制台（延迟1秒，服务有时间启动）
    QTimer::singleShot(1000, this, &FinishPage::openDashboard);
}

void FinishPage::openDashboard()
{
    auto *wiz = qobject_cast<DeployWizard *>(wizard());
    if (!wiz) return;

    const QString url = wiz->deployEngine()->dashboardUrl();
    QDesktopServices::openUrl(QUrl(url));
}
