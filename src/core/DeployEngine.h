#pragma once
#include <QObject>
#include <QString>
#include <QStringList>
#include <QProcess>

struct DeployConfig {
    QString installPath;
    int     servicePort     = 8080;
    QString adminPassword;
    QString domainName;
    bool    installService  = true;
    bool    startOnBoot     = true;

    // 平台集成
    bool    enableWx        = false;
    QString wxWebhook;
    bool    enableQq        = false;
    QString qqWebhook;
    bool    enableDt        = false;
    QString dtWebhook;
    bool    enableFs        = false;
    QString fsWebhook;
};

class DeployEngine : public QObject {
    Q_OBJECT
public:
    explicit DeployEngine(QObject *parent = nullptr);

    void setConfig(const DeployConfig &cfg) { m_config = cfg; }
    const DeployConfig &config() const { return m_config; }

    // 异步部署入口，进度通过信号推送
    void startDeploy();

    // 查询是否已有服务运行
    bool isServiceRunning() const;

    // 获取服务 URL
    QString dashboardUrl() const;

signals:
    void progressChanged(int percent, const QString &message);
    void deployed();
    void deployFailed(const QString &reason);

private:
    bool extractBinaries();
    bool writeConfig();
    bool installSystemService();
    bool startService();
    bool writePlatformConfig();

    QString platformServiceExe() const;
    QString serviceUnitPath() const;   // Linux only

    DeployConfig m_config;
    QProcess    *m_proc = nullptr;
};
