#include "InstallConfigPage.h"
#include "../DeployWizard.h"

#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QLabel>
#include <QPushButton>
#include <QFileDialog>
#include <QDir>
#include <QStandardPaths>
#include <QMessageBox>

InstallConfigPage::InstallConfigPage(QWidget *parent)
    : QWizardPage(parent)
{
    setTitle(tr("安装路径"));
    setSubTitle(tr("选择 OpenClaw 服务程序的安装位置。"));

    auto *root = new QVBoxLayout(this);
    root->setSpacing(14);

    // 路径选择
    auto *pathGroup = new QVBoxLayout;
    pathGroup->addWidget(new QLabel(tr("安装目录："), this));

    auto *pathRow = new QHBoxLayout;
    m_pathEdit = new QLineEdit(this);
    m_pathEdit->setPlaceholderText(tr("请选择目录…"));
    auto *browseBtn = new QPushButton(tr("浏览…"), this);
    browseBtn->setFixedWidth(80);
    pathRow->addWidget(m_pathEdit);
    pathRow->addWidget(browseBtn);
    pathGroup->addLayout(pathRow);

    auto *pathNote = new QLabel(
        tr("推荐选择非系统盘且路径中不含中文或空格的目录。"), this);
    pathNote->setStyleSheet("color: #666; font-size: 11px;");
    pathGroup->addWidget(pathNote);
    root->addLayout(pathGroup);

    root->addSpacing(8);

    // 服务选项
    m_installService = new QCheckBox(tr("注册为系统服务（推荐）"), this);
    m_installService->setChecked(true);
    m_startOnBoot = new QCheckBox(tr("开机自动启动服务"), this);
    m_startOnBoot->setChecked(true);

    root->addWidget(m_installService);
    root->addWidget(m_startOnBoot);

    connect(m_installService, &QCheckBox::toggled, m_startOnBoot, &QCheckBox::setEnabled);

    root->addStretch();

    // 注册字段（供后续页面 wizard()->field() 读取）
    registerField(ConfigKey::InstallPath + QStringLiteral("*"), m_pathEdit);
    registerField(ConfigKey::InstallService, m_installService);
    registerField(ConfigKey::StartOnBoot, m_startOnBoot);

    connect(browseBtn, &QPushButton::clicked, this, [this] {
        QString dir = QFileDialog::getExistingDirectory(
            this, tr("选择安装目录"), m_pathEdit->text());
        if (!dir.isEmpty())
            m_pathEdit->setText(QDir::toNativeSeparators(dir));
    });
}

void InstallConfigPage::initializePage()
{
    if (m_pathEdit->text().isEmpty()) {
#if defined(Q_OS_LINUX)
        m_pathEdit->setText("/opt/openclaw");
#elif defined(Q_OS_WIN)
        QString prog = QStandardPaths::writableLocation(QStandardPaths::AppLocalDataLocation);
        m_pathEdit->setText(QDir::toNativeSeparators(prog + "/openclaw"));
#endif
    }
}

bool InstallConfigPage::validatePage()
{
    QString path = m_pathEdit->text().trimmed();
    if (path.isEmpty()) {
        QMessageBox::warning(this, tr("提示"), tr("请指定安装目录。"));
        return false;
    }
    QDir d(path);
    if (!d.exists()) {
        auto ret = QMessageBox::question(this, tr("目录不存在"),
            tr("目录 %1 不存在，是否自动创建？").arg(path));
        if (ret != QMessageBox::Yes)
            return false;
        if (!d.mkpath(".")) {
            QMessageBox::critical(this, tr("错误"), tr("无法创建目录，请检查权限。"));
            return false;
        }
    }
    return true;
}
