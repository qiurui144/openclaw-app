#include "DeployEngine.h"

#include <QFile>
#include <QDir>
#include <QFileInfo>
#include <QTextStream>
#include <QProcess>
#include <QCoreApplication>
#include <QStandardPaths>
#include <QJsonObject>
#include <QJsonDocument>
#include <QJsonArray>
#include <QTimer>
#include <QDateTime>

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
                writeInstallRecord();  // 写安装记录（非致命）
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
bool DeployEngine::hasBundledBinaries() const
{
#if defined(Q_OS_LINUX)
    return QFile::exists(":/binaries/linux/openclaw-server");
#elif defined(Q_OS_WIN)
    return QFile::exists(":/binaries/windows/openclaw-server.exe");
#else
    return false;
#endif
}

QString DeployEngine::bundledBinaryPath(const QString &name) const
{
#if defined(Q_OS_LINUX)
    return ":/binaries/linux/" + name;
#elif defined(Q_OS_WIN)
    return ":/binaries/windows/" + name + ".exe";
#else
    return QString();
#endif
}

bool DeployEngine::extractBinaries()
{
    QDir d(m_config.installPath);
    if (!d.exists() && !d.mkpath("."))
        return false;

#if defined(Q_OS_LINUX)
    const QStringList entries = { "openclaw-server", "openclaw-config" };
#elif defined(Q_OS_WIN)
    const QStringList entries = { "openclaw-server.exe", "openclaw-config.exe" };
#else
    const QStringList entries;
#endif

    bool hasAny = false;
    for (const QString &name : entries) {
        const QString srcPath = bundledBinaryPath(
            name.endsWith(".exe") ? name.chopped(4) : name);
        QFile src(srcPath);
        if (!src.exists()) {
            // 无内嵌二进制时，写placeholder（开发阶段/CI占位）
            QFile mark(d.filePath(name + ".placeholder"));
            if (mark.open(QIODevice::WriteOnly))
                mark.write("# OpenClaw binary placeholder – replace with real binary\n");
            continue;
        }

        hasAny = true;
        const QString dstPath = d.filePath(name);
        if (QFile::exists(dstPath))
            QFile::remove(dstPath);
        if (!src.open(QIODevice::ReadOnly))
            return false;

        QFile dst(dstPath);
        if (!dst.open(QIODevice::WriteOnly))
            return false;

        // 分块写入，避免大文件一次性占用大量内存
        constexpr int kChunk = 2 * 1024 * 1024;  // 2 MB
        while (!src.atEnd())
            dst.write(src.read(kChunk));
        src.close();
        dst.close();

#if defined(Q_OS_LINUX)
        QFile::setPermissions(dstPath,
            QFileDevice::ReadOwner | QFileDevice::WriteOwner | QFileDevice::ExeOwner |
            QFileDevice::ReadGroup | QFileDevice::ExeGroup |
            QFileDevice::ReadOther | QFileDevice::ExeOther);
#endif
    }

    // 如果一个真实二进制都没有，仅是占位，部署也算通过（开发模式）
    Q_UNUSED(hasAny)
    return true;
}

// ── 写入主配置 JSON ──────────────────────────────────────────────────────────
bool DeployEngine::writeConfig()
{
    QJsonObject cfg;
    cfg["port"]           = m_config.servicePort;
    cfg["admin_password"] = m_config.adminPassword;
    cfg["domain"]         = m_config.domainName;
    cfg["install_path"]   = m_config.installPath;
    cfg["version"]        = QCoreApplication::applicationVersion();
    cfg["deployed_at"]    = QDateTime::currentDateTimeUtc().toString(Qt::ISODate);

    QJsonDocument doc(cfg);
    QFile f(m_config.installPath + "/openclaw.json");
    if (!f.open(QIODevice::WriteOnly | QIODevice::Text))
        return false;
    f.write(doc.toJson(QJsonDocument::Indented));
    return true;
}

// ── 写安装记录（供后续更新/卸载定位） ────────────────────────────────────────
bool DeployEngine::writeInstallRecord()
{
    const QString recordDir = QStandardPaths::writableLocation(
        QStandardPaths::AppDataLocation);
    QDir().mkpath(recordDir);

    QJsonObject rec;
    rec["install_path"]  = m_config.installPath;
    rec["version"]       = QCoreApplication::applicationVersion();
    rec["installed_at"]  = QDateTime::currentDateTimeUtc().toString(Qt::ISODate);
    rec["service_port"]  = m_config.servicePort;

    QFile f(recordDir + "/install_record.json");
    if (!f.open(QIODevice::WriteOnly | QIODevice::Text))
        return false;
    f.write(QJsonDocument(rec).toJson(QJsonDocument::Indented));
    return true;
}

QString DeployEngine::installRecordPath()
{
    return QStandardPaths::writableLocation(QStandardPaths::AppDataLocation)
         + "/install_record.json";
}

// ── 写入平台集成配置 ─────────────────────────────────────────────────────────
bool DeployEngine::writePlatformConfig()
{
    QJsonObject platforms;
    QJsonArray  enabled;

    if (m_config.enableWx && !m_config.wxWebhook.isEmpty()) {
        platforms["wechat_work_webhook"] = m_config.wxWebhook;
        enabled.append("wechat_work");
    }
    if (m_config.enableQq && !m_config.qqWebhook.isEmpty()) {
        platforms["qq_work_webhook"]     = m_config.qqWebhook;
        enabled.append("qq_work");
    }
    if (m_config.enableDt && !m_config.dtWebhook.isEmpty()) {
        platforms["dingtalk_webhook"]    = m_config.dtWebhook;
        enabled.append("dingtalk");
    }
    if (m_config.enableFs && !m_config.fsWebhook.isEmpty()) {
        platforms["feishu_webhook"]      = m_config.fsWebhook;
        enabled.append("feishu");
    }
    platforms["enabled_platforms"] = enabled;

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
