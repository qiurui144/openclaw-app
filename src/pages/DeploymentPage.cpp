#include "DeploymentPage.h"
#include "../DeployWizard.h"
#include "../core/DeployEngine.h"

#include <QVBoxLayout>
#include <QDateTime>

DeploymentPage::DeploymentPage(QWidget *parent)
    : QWizardPage(parent)
{
    setTitle(tr("正在部署"));
    setSubTitle(tr("OpenClaw 服务部署进行中，请勿关闭本窗口…"));
    setCommitPage(true);   // 不允许返回

    auto *root = new QVBoxLayout(this);
    root->setSpacing(10);

    m_statusLabel = new QLabel(tr("准备中…"), this);
    m_statusLabel->setAlignment(Qt::AlignCenter);

    m_progressBar = new QProgressBar(this);
    m_progressBar->setRange(0, 100);
    m_progressBar->setValue(0);
    m_progressBar->setTextVisible(true);
    m_progressBar->setFixedHeight(24);

    m_logView = new QTextEdit(this);
    m_logView->setReadOnly(true);
    m_logView->setObjectName("logView");
    m_logView->setFont(QFont("Monospace", 9));
    m_logView->setMinimumHeight(160);

    root->addWidget(m_statusLabel);
    root->addWidget(m_progressBar);
    root->addWidget(m_logView);
}

void DeploymentPage::initializePage()
{
    m_done   = false;
    m_failed = false;
    m_progressBar->setValue(0);
    m_logView->clear();
    setSubTitle(tr("OpenClaw 服务部署进行中，请勿关闭本窗口…"));

    collectConfigFromWizard();

    auto *wiz    = qobject_cast<DeployWizard *>(wizard());
    auto *engine = wiz ? wiz->deployEngine() : nullptr;
    if (!engine) {
        onDeployFailed(tr("内部错误：无法获取部署引擎"));
        return;
    }

    connect(engine, &DeployEngine::progressChanged,
            this, &DeploymentPage::onProgress, Qt::UniqueConnection);
    connect(engine, &DeployEngine::deployed,
            this, &DeploymentPage::onDeployed, Qt::UniqueConnection);
    connect(engine, &DeployEngine::deployFailed,
            this, &DeploymentPage::onDeployFailed, Qt::UniqueConnection);

    engine->startDeploy();
}

void DeploymentPage::collectConfigFromWizard()
{
    auto *wiz    = qobject_cast<DeployWizard *>(wizard());
    if (!wiz) return;

    DeployConfig cfg;
    cfg.installPath    = wiz->field(ConfigKey::InstallPath).toString();
    cfg.servicePort    = wiz->field(ConfigKey::ServicePort).toInt();
    cfg.adminPassword  = wiz->field(ConfigKey::AdminPassword).toString();
    cfg.domainName     = wiz->field(ConfigKey::DomainName).toString();
    cfg.installService = wiz->field(ConfigKey::InstallService).toBool();
    cfg.startOnBoot    = wiz->field(ConfigKey::StartOnBoot).toBool();

    cfg.enableWx  = wiz->field(ConfigKey::EnableWx).toBool();
    cfg.wxWebhook = wiz->field(ConfigKey::WxWebhook).toString();
    cfg.enableQq  = wiz->field(ConfigKey::EnableQq).toBool();
    cfg.qqWebhook = wiz->field(ConfigKey::QqWebhook).toString();
    cfg.enableDt  = wiz->field(ConfigKey::EnableDt).toBool();
    cfg.dtWebhook = wiz->field(ConfigKey::DtWebhook).toString();
    cfg.enableFs  = wiz->field(ConfigKey::EnableFs).toBool();
    cfg.fsWebhook = wiz->field(ConfigKey::FsWebhook).toString();

    wiz->deployEngine()->setConfig(cfg);
}

void DeploymentPage::onProgress(int pct, const QString &msg)
{
    m_progressBar->setValue(pct);
    m_statusLabel->setText(msg);
    const QString ts = QDateTime::currentDateTime().toString("hh:mm:ss");
    m_logView->append(QString("[%1] %2").arg(ts, msg));
}

void DeploymentPage::onDeployed()
{
    m_done = true;
    setSubTitle(tr("部署成功！点击完成并启动以打开控制台。"));
    emit completeChanged();
}

void DeploymentPage::onDeployFailed(const QString &reason)
{
    m_failed = true;
    m_done   = true;
    m_progressBar->setStyleSheet("QProgressBar::chunk { background: #c62828; }");
    m_statusLabel->setText(tr("部署失败：%1").arg(reason));
    const QString ts = QDateTime::currentDateTime().toString("hh:mm:ss");
    m_logView->append(QString("[%1] ✗ 失败：%2").arg(ts, reason));
    setSubTitle(tr("部署过程中发生错误，请查看日志并排查原因。"));
    emit completeChanged();
}

bool DeploymentPage::isComplete() const
{
    return m_done;
}
