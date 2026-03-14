#pragma once
#include <QObject>
#include <QUrl>
#include <QString>
#include <QNetworkAccessManager>
#include <QNetworkReply>

// ── UpdateChecker ────────────────────────────────────────────
// 向更新服务器查询最新版本，若有更新则下载并替换当前 AppImage。
//
// 服务端接口约定（GET）：
//   ${OC_UPDATE_URL}/version  →  {"version":"1.1.0","url":"...","sha256":"..."}
//
// 使用示例：
//   UpdateChecker *uc = new UpdateChecker(this);
//   uc->check(QUrl("https://update.example.com"));
//
class UpdateChecker : public QObject {
    Q_OBJECT
public:
    explicit UpdateChecker(QObject *parent = nullptr);

    // 向 baseUrl/version 发请求，异步返回结果
    void check(const QUrl &baseUrl, int timeoutMs = 8000);

signals:
    void updateAvailable(const QString &version, const QUrl &downloadUrl,
                         const QString &sha256);
    void upToDate();
    void checkFailed(const QString &reason);

    // 下载进度 0-100
    void downloadProgress(int percent);
    // 下载完成，已替换 AppImage，需重启
    void downloadFinished();
    void downloadFailed(const QString &reason);

public slots:
    // 下载并替换 AppImage（downloadUrl 来自 updateAvailable 信号）
    void downloadAndReplace(const QUrl &downloadUrl, const QString &sha256 = {});

private slots:
    void onVersionReply(QNetworkReply *reply);
    void onDownloadReadyRead();
    void onDownloadFinished();

private:
    bool replaceAppImage(const QString &tmpPath, const QString &sha256);

    QNetworkAccessManager *m_nam     = nullptr;
    QNetworkReply         *m_dlReply = nullptr;
    QString                m_tmpPath;
    QString                m_expectedSha256;
};
