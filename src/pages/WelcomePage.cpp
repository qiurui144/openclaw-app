#include "WelcomePage.h"
#include <QLabel>
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QFrame>
#include <QPushButton>
#include <QFont>
#include <QApplication>
#include <QDesktopServices>
#include <QUrl>

WelcomePage::WelcomePage(QWidget *parent)
    : QWizardPage(parent)
{
    setTitle(tr("欢迎使用 OpenClaw 一键部署向导"));
    setSubTitle(tr("本向导将引导您在本机完成 OpenClaw 服务的完整部署，全程无需访问境外网络。"));

    auto *root = new QVBoxLayout(this);
    root->setSpacing(12);

    // 功能亮点卡片
    auto addFeatureRow = [&](const QString &icon, const QString &text) {
        auto *row = new QHBoxLayout;
        auto *ico = new QLabel(icon, this);
        ico->setFixedWidth(32);
        QFont f = ico->font();
        f.setPixelSize(20);
        ico->setFont(f);
        ico->setAlignment(Qt::AlignCenter);

        auto *lbl = new QLabel(text, this);
        lbl->setWordWrap(true);
        row->addWidget(ico);
        row->addWidget(lbl, 1);
        root->addLayout(row);
    };

    root->addSpacing(8);
    addFeatureRow("📦", tr("<b>全量内置</b> — 所有二进制文件已预置，无需服务器或翻墙下载"));
    root->addSpacing(4);
    addFeatureRow("🔗", tr("<b>平台集成</b> — 支持企业微信、QQ Work、钉钉、飞书一键配置"));
    root->addSpacing(4);
    addFeatureRow("🚀", tr("<b>开机自启</b> — 可注册为系统服务，重启后自动恢复运行"));
    root->addSpacing(4);
    addFeatureRow("🌐", tr("<b>网页控制台</b> — 部署完成后自动打开本地管理界面"));
    root->addSpacing(12);

    // 分割线
    auto *line = new QFrame(this);
    line->setFrameShape(QFrame::HLine);
    line->setFrameShadow(QFrame::Sunken);
    root->addWidget(line);
    root->addSpacing(4);

    auto *note = new QLabel(
        tr("点击 <b>下一步</b> 开始部署前置检查。如需帮助，请访问 "
           "<a href='https://github.com/openclaw'>github.com/openclaw</a>"),
        this);
    note->setOpenExternalLinks(true);
    note->setWordWrap(true);
    root->addWidget(note);
    root->addStretch();
}

void WelcomePage::initializePage()
{
    // Welcome 页不需要特殊初始化
}
