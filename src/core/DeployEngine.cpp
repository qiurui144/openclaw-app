#include "DeployEngine.h"

#include <QFile>
#include <QDir>
#include <QFileInfo>
#include <QTextStream>
#include <QProcess>
#include <QCoreApplication>
#include <QThread>
#include <QJsonObject>
#include <QJsonDocument>
#include <QTimer>

#ifdef Q_OS_WIN
#  include <windows.h>
#endif

DeployEngine::DeployEngine(QObject *parent)
    : QObject(parent)
{}

void DeployEngine::startDeploy()
{
    // 使用计时器模拟异步步骤（每步用 invokeMethod/Lambda 链式执行）
    auto step = [this](int pct, const QString &msg, auto &&fn) {
        emit progressChanged(pct, msg);
        QTimer::singleShot(300, this, fn);
    };

    emit progressChanged(0, tr("准备部署目录…"));

    QTimer::singleShot(200, this, [this, step] {
        // Step 1: 创建目录
        QDir d(m_config.installPath);
        if (!d.exists() && !d.mkpath(".")) {
            emit deployFailed(tr("无法创建目录: %1").arg(m_config.installPath));
            return;
        }
        emit progressChanged(15, tr("提取服务程序…"));

        QTimer::singleShot(400, this, [this] {
            // Step 2: 提取二进制
            if (!extractBinaries()) {
                emit deployFailed(tr("提取程序文件失败，请检查磁盘空间和权限"));
                return;
            }
            emit progressChanged(40, tr("写入配置文件…"));

            QTimer::singleShot(300, this, [this] {
                // Step 3: 写入配置
                if (!writeConfig()) {
                    emit deployFailed(tr("写入配置文件失败"));
                    return;
                }
                if (!writePlatformConfig()) {
                    emit deployFailed(tr("写入平台集成配置失败"));
                    return;
                }
                emit progressChanged(60, tr("注册系统服务…"));

                QTimer::singleShot(600, this, [this] {
                    // Step 4: 安装系统服务
                    if (m_config.installService && !installSystemService()) {
                        // 服务安装失败不是致命错误，仅警告
                        emit progressChanged(75, tr("服务注册跳过（可手动启动）"));
                    } else {
                        emit progressChanged(75, tr("启动服务…"));
                    }

                    QTimer::singleShot(800, this, [this] {
                        // Step 5: 启动服务
                        if (!startService()) {
                            // 启动失败也不阻断，日志记录
                            emit progressChanged(90, tr("服务启动失败，请手动启动"));
                        } else {
                            emit progressChanged(90, tr("服务启动成功"));
                        }

                        QTimer::singleShot(500, this, [this] {
                            emit progressChanged(100, tr("部署完成！"));
                            emit deployed();
                        });
                    });
                });
            });
        });
    });
}

// ── 提取内嵌二进制 ─────────────────────────────────────────────────────────
bool DeployEngine::extractBinaries()
{
    QDir d(m_config.installPath);
    if (!d.exists() && !d.mkpath("."))
        return false;

#if defined(Q_OS_LINUX)
    const QStringList files = {
        "linux/openclaw-server",
        "linux/openclaw-config",
    };
#elif defined(Q_OS_WIN)
    const QStringList files = {
        "windows/openclaw-server.exe",
        "windows/openclaw-config.exe",
    };
#else
    const QStringList files;
#endif

    for (const QString &rel : files) {
        const QString srcPath = ":/binaries/" + rel;
        QFile src(srcPath);
        if (!src.exists()) {
            // 二进制占位文件不存在时写一个标记文件（便于开发阶段调试）
            QFile mark(d.filePath(QFileInfo(rel).fileName() + ".placeholder"));
            mark.open(QIODevice::WriteOnly);
            mark.write("# OpenClaw binary placeholder\n");
            mark.close();
            continue;
        }
        const QString dstPath = d.filePath(QFileInfo(rel).fileName());
        if (QFile::exists(dstPath))
            QFile::remove(dstPath);
        if (!src.open(QIODevice::ReadOnly))
            return false;
        QFile dst(dstPath);
        if (!dst.open(QIODevice::WriteOnly))
            return false;
        while (!src.atEnd())
            dst.write(src.read(1024 * 1024));
        src.close();
        dst.close();

#if defined(Q_OS_LINUX)
        QFile::setPermissions(dstPath,
            QFileDevice::ReadOwner | QFileDevice::WriteOwner | QFileDevice::ExeOwner |
            QFileDevice::ReadGroup | QFileDevice::ExeGroup |
            QFileDevice::ReadOther | QFileDevice::ExeOther);
#endif
    }
    return true;
}

// ── 写入主配置 JSON ──────────────────────────────────────────────────────────
bool DeployEngine::writeConfig()
{
    QJsonObject cfg;
    cfg["port"]          = m_config.servicePort;
    cfg["admin_password"] = m_config.adminPassword;
    cfg["domain"]        = m_config.domainName;
    cfg["install_path"]  = m_config.installPath;
    cfg["version"]       = QCoreApplication::applicationVersion();

    QJsonDocument doc(cfg);
    QFile f(m_config.installPath + "/openclaw.json");
    if (!f.open(QIODevice::WriteOnly | QIODevice::Text))
        return false;
    f.write(doc.toJson(QJsonDocument::Indented));
    return true;
}

// ── 写入平台集成配置 ─────────────────────────────────────────────────────────
bool DeployEngine::writePlatformConfig()
{
    QJsonObject platforms;
    if (m_config.enableWx)
        platforms["wechat_work_webhook"] = m_config.wxWebhook;
    if (m_config.enableQq)
        platforms["qq_work_webhook"]     = m_config.qqWebhook;
    if (m_config.enableDt)
        platforms["dingtalk_webhook"]    = m_config.dtWebhook;
    if (m_config.enableFs)
        platforms["feishu_webhook"]      = m_config.fsWebhook;

    QJsonDocument doc(platforms);
    QFile f(m_config.installPath + "/platforms.json");
    if (!f.open(QIODevice::WriteOnly | QIODevice::Text))
        return false;
    f.write(doc.toJson(QJsonDocument::Indented));
    return true;
}

// ── 注册系统服务 ─────────────────────────────────────────────────────────────
bool DeployEngine::installSystemService()
{
#if defined(Q_OS_LINUX)
    const QString unitContent = QString(
        "[Unit]\n"
        "Description=OpenClaw Service\n"
        "After=network.target\n\n"
        "[Service]\n"
        "Type=simple\n"
        "ExecStart=%1/openclaw-server --config %1/openclaw.json\n"
        "Restart=on-failure\n"
        "RestartSec=5\n"
        "StandardOutput=journal\n"
        "StandardError=journal\n\n"
        "[Install]\n"
        "WantedBy=multi-user.target\n"
    ).arg(m_config.installPath);

    {
        QFile unit("/etc/systemd/system/openclaw.service");
        if (!unit.open(QIODevice::WriteOnly | QIODevice::Text))
            return false;
        QTextStream(&unit) << unitContent;
    }

    QProcess p;
    p.start("systemctl", {"daemon-reload"});
    p.waitForFinished(5000);
    if (m_config.startOnBoot) {
        p.start("systemctl", {"enable", "openclaw.service"});
        p.waitForFinished(5000);
    }
    return true;

#elif defined(Q_OS_WIN)
    const QString exePath = QDir::toNativeSeparators(
        m_config.installPath + "/openclaw-server.exe");
    QProcess p;
    p.start("sc", {"create", "openclaw",
                   "binPath=", exePath,
                   "start=", m_config.startOnBoot ? "auto" : "demand",
                   "DisplayName=", "OpenClaw Service"});
    p.waitForFinished(10000);
    return p.exitCode() == 0;
#else
    return false;
#endif
}

// ── 启动服务 ─────────────────────────────────────────────────────────────────
bool DeployEngine::startService()
{
#if defined(Q_OS_LINUX)
    QProcess p;
    p.start("systemctl", {"start", "openclaw.service"});
    p.waitForFinished(10000);
    return p.exitCode() == 0;

#elif defined(Q_OS_WIN)
    QProcess p;
    p.start("sc", {"start", "openclaw"});
    p.waitForFinished(10000);
    return p.exitCode() == 0;

#else
    return false;
#endif
}

bool DeployEngine::isServiceRunning() const
{
#if defined(Q_OS_LINUX)
    QProcess p;
    p.start("systemctl", {"is-active", "--quiet", "openclaw.service"});
    p.waitForFinished(3000);
    return p.exitCode() == 0;
#elif defined(Q_OS_WIN)
    QProcess p;
    p.start("sc", {"query", "openclaw"});
    p.waitForFinished(3000);
    return p.readAllStandardOutput().contains("RUNNING");
#else
    return false;
#endif
}

QString DeployEngine::dashboardUrl() const
{
    const QString host = m_config.domainName.isEmpty()
                       ? "127.0.0.1"
                       : m_config.domainName;
    return QString("http://%1:%2").arg(host).arg(m_config.servicePort);
}
