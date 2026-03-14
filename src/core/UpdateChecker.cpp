#include "UpdateChecker.h"

#include <QJsonDocument>
#include <QJsonObject>
#include <QFile>
#include <QFileInfo>
#include <QDir>
#include <QProcess>
#include <QCoreApplication>
#include <QTimer>
#include <QCryptographicHash>

#ifndef OC_VERSION
#  define OC_VERSION "0.0.0"
#endif

// ── 版本比较（major.minor.patch 数值比较）────────────────────
static bool isNewerVersion(const QString &serverVer, const QString &localVer)
{
    auto parts = [](const QString &v) -> QList<int> {
        QList<int> r;
        for (const auto &p : v.split('.'))
            r << p.toInt();
        while (r.size() < 3) r << 0;
        return r;
    };
    auto sv = parts(serverVer);
    auto lv = parts(localVer);
    for (int i = 0; i < 3; ++i) {
        if (sv[i] > lv[i]) return true;
        if (sv[i] < lv[i]) return false;
    }
    return false;
}

UpdateChecker::UpdateChecker(QObject *parent)
    : QObject(parent)
    , m_nam(new QNetworkAccessManager(this))
{}

void UpdateChecker::check(const QUrl &baseUrl, int timeoutMs)
{
    QUrl versionUrl = baseUrl.url().endsWith('/')
        ? QUrl(baseUrl.toString() + "version")
        : QUrl(baseUrl.toString() + "/version");

    QNetworkRequest req(versionUrl);
    req.setHeader(QNetworkRequest::UserAgentHeader,
                  QString("OpenClaw/%1").arg(OC_VERSION));
    req.setRawHeader("Accept", "application/json");

    QNetworkReply *reply = m_nam->get(req);

    // 超时保护
    QTimer *timer = new QTimer(reply);
    timer->setSingleShot(true);
    connect(timer, &QTimer::timeout, reply, [reply] {
        reply->abort();
    });
    timer->start(timeoutMs);

    connect(reply, &QNetworkReply::finished,
            this,  [this, reply] { onVersionReply(reply); });
}

void UpdateChecker::onVersionReply(QNetworkReply *reply)
{
    reply->deleteLater();

    if (reply->error() != QNetworkReply::NoError) {
        emit checkFailed(reply->errorString());
        return;
    }

    QByteArray data = reply->readAll();
    QJsonParseError err;
    QJsonDocument doc = QJsonDocument::fromJson(data, &err);
    if (err.error != QJsonParseError::NoError || !doc.isObject()) {
        emit checkFailed(tr("服务器返回格式错误: %1").arg(err.errorString()));
        return;
    }

    QJsonObject obj = doc.object();
    QString serverVersion = obj.value("version").toString();
    QString urlStr        = obj.value("url").toString();
    QString sha256        = obj.value("sha256").toString();

    if (serverVersion.isEmpty()) {
        emit checkFailed(tr("服务器未返回版本号"));
        return;
    }

    if (isNewerVersion(serverVersion, QStringLiteral(OC_VERSION))) {
        emit updateAvailable(serverVersion, QUrl(urlStr), sha256);
    } else {
        emit upToDate();
    }
}

void UpdateChecker::downloadAndReplace(const QUrl &downloadUrl,
                                       const QString &sha256)
{
    m_expectedSha256 = sha256;

    // 临时文件放在系统临时目录
    m_tmpPath = QDir::tempPath() + "/openclaw-update.AppImage";

    QFile *tmpFile = new QFile(m_tmpPath, this);
    if (!tmpFile->open(QIODevice::WriteOnly | QIODevice::Truncate)) {
        emit downloadFailed(tr("无法写入临时文件: %1").arg(m_tmpPath));
        return;
    }

    QNetworkRequest req(downloadUrl);
    m_dlReply = m_nam->get(req);

    connect(m_dlReply, &QNetworkReply::readyRead, this, [this, tmpFile] {
        tmpFile->write(m_dlReply->readAll());
    });

    connect(m_dlReply, &QNetworkReply::downloadProgress,
            this, [this](qint64 recv, qint64 total) {
        if (total > 0)
            emit downloadProgress(static_cast<int>(recv * 100 / total));
    });

    connect(m_dlReply, &QNetworkReply::finished, this, [this, tmpFile] {
        tmpFile->close();
        onDownloadFinished();
    });
}

void UpdateChecker::onDownloadReadyRead()  {}   // handled by lambda above
void UpdateChecker::onDownloadFinished()
{
    m_dlReply->deleteLater();

    if (m_dlReply->error() != QNetworkReply::NoError) {
        QFile::remove(m_tmpPath);
        emit downloadFailed(m_dlReply->errorString());
        return;
    }

    if (!replaceAppImage(m_tmpPath, m_expectedSha256)) {
        QFile::remove(m_tmpPath);
        return;   // replaceAppImage already emits downloadFailed
    }

    emit downloadFinished();
}

bool UpdateChecker::replaceAppImage(const QString &tmpPath,
                                    const QString &sha256)
{
    // SHA-256 校验（若服务端提供）
    if (!sha256.isEmpty()) {
        QFile f(tmpPath);
        if (!f.open(QIODevice::ReadOnly)) {
            emit downloadFailed(tr("无法读取下载文件进行校验"));
            return false;
        }
        QCryptographicHash hash(QCryptographicHash::Sha256);
        hash.addData(&f);
        f.close();
        QString actual = hash.result().toHex();
        if (actual.compare(sha256, Qt::CaseInsensitive) != 0) {
            emit downloadFailed(
                tr("文件校验失败（预期: %1  实际: %2）").arg(sha256, actual));
            return false;
        }
    }

    // 找到当前 AppImage 路径（AppImage 运行时会设置 $APPIMAGE 环境变量）
    QString appImagePath = qEnvironmentVariable("APPIMAGE");
    if (appImagePath.isEmpty()) {
        // 非 AppImage 环境下（开发调试）：直接替换可执行文件
        appImagePath = QCoreApplication::applicationFilePath();
    }

    // 赋予可执行权限
    QFile::setPermissions(tmpPath,
        QFile::ReadOwner | QFile::WriteOwner | QFile::ExeOwner |
        QFile::ReadGroup | QFile::ExeGroup |
        QFile::ReadOther  | QFile::ExeOther);

    // 替换：先删旧文件再移动（跨文件系统用 copy+remove）
    QFile oldFile(appImagePath);
    QString backupPath = appImagePath + ".bak";
    oldFile.rename(backupPath);                   // rename 到备份

    if (!QFile::rename(tmpPath, appImagePath)) {
        // rename 跨设备失败 → 逐块复制
        if (!QFile::copy(tmpPath, appImagePath)) {
            // 回滚
            QFile::rename(backupPath, appImagePath);
            QFile::remove(tmpPath);
            emit downloadFailed(tr("替换 AppImage 失败，已回滚"));
            return false;
        }
        QFile::remove(tmpPath);
        QFile::setPermissions(appImagePath,
            QFile::ReadOwner | QFile::WriteOwner | QFile::ExeOwner |
            QFile::ReadGroup | QFile::ExeGroup |
            QFile::ReadOther  | QFile::ExeOther);
    }

    QFile::remove(backupPath);
    return true;
}
