#include "SystemCheckPage.h"
#include "../core/SystemCheck.h"

#include <QHBoxLayout>
#include <QLabel>
#include <QFrame>
#include <QTimer>

SystemCheckPage::SystemCheckPage(QWidget *parent)
    : QWizardPage(parent)
{
    setTitle(tr("系统环境检查"));
    setSubTitle(tr("正在检查运行环境，请稍候…"));

    auto *root = new QVBoxLayout(this);

    m_listLayout = new QVBoxLayout;
    m_listLayout->setSpacing(8);
    root->addLayout(m_listLayout);
    root->addStretch();

    m_summaryLabel = new QLabel(this);
    m_summaryLabel->setWordWrap(true);
    root->addWidget(m_summaryLabel);
}

void SystemCheckPage::initializePage()
{
    // 清空旧结果
    while (QLayoutItem *item = m_listLayout->takeAt(0)) {
        delete item->widget();
        delete item;
    }
    m_allRequired = false;
    m_summaryLabel->clear();
    setSubTitle(tr("正在检查运行环境，请稍候…"));

    QTimer::singleShot(200, this, &SystemCheckPage::runChecks);
}

void SystemCheckPage::runChecks()
{
    SystemCheck checker;
    QList<CheckItem> items = checker.runAll();

    bool allOk = true;
    for (const CheckItem &item : items) {
        addCheckRow(item);
        if (item.required && !item.passed)
            allOk = false;
    }
    m_allRequired = allOk;

    if (allOk) {
        setSubTitle(tr("所有必要项检查通过，可以继续安装。"));
        m_summaryLabel->setText(
            tr("<span style='color:#2e7d32;font-weight:bold;'>"
               "✓ 环境检查通过，点击下一步继续。</span>"));
    } else {
        setSubTitle(tr("部分必要项未通过，请根据提示修复后重试。"));
        m_summaryLabel->setText(
            tr("<span style='color:#c62828;font-weight:bold;'>"
               "\u2717 \u5b58\u5728\u672a\u6ee1\u8db3\u7684\u5fc5\u8981\u6761\u4ef6\uff0c"
               "\u8bf7\u4fee\u590d\u540e\u70b9\u51fb\u300a\u4e0a\u4e00\u6b65\u300b"
               "\u91cd\u65b0\u8fd0\u884c\u68c0\u67e5\u3002</span>"));
    }
    emit completeChanged();
}

void SystemCheckPage::addCheckRow(const CheckItem &item)
{
    auto *frame = new QFrame(this);
    frame->setFrameShape(QFrame::StyledPanel);
    frame->setObjectName(item.passed ? "checkPass" : (item.required ? "checkFail" : "checkWarn"));

    auto *row = new QHBoxLayout(frame);
    row->setContentsMargins(8, 6, 8, 6);

    auto *icon = new QLabel(frame);
    icon->setFixedWidth(24);
    icon->setAlignment(Qt::AlignCenter);
    if (item.passed) {
        icon->setText("✓");
        icon->setStyleSheet("color:#2e7d32; font-weight:bold; font-size:14px;");
    } else if (item.required) {
        icon->setText("✗");
        icon->setStyleSheet("color:#c62828; font-weight:bold; font-size:14px;");
    } else {
        icon->setText("⚠");
        icon->setStyleSheet("color:#e65100; font-weight:bold; font-size:14px;");
    }

    auto *nameLabel   = new QLabel(item.name,   frame);
    auto *detailLabel = new QLabel(item.detail, frame);
    detailLabel->setStyleSheet("color:#555;");
    detailLabel->setWordWrap(true);

    row->addWidget(icon);
    row->addWidget(nameLabel, 1);
    row->addWidget(detailLabel, 2);

    m_listLayout->addWidget(frame);
}

bool SystemCheckPage::isComplete() const
{
    return m_allRequired;
}
