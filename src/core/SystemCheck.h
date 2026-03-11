#pragma once
#include <QObject>
#include <QString>
#include <QList>

struct CheckItem {
    QString name;
    QString detail;
    bool    passed = false;
    bool    required = true;   // false = 仅警告
};

class SystemCheck : public QObject {
    Q_OBJECT
public:
    explicit SystemCheck(QObject *parent = nullptr);

    // 执行所有检查，返回检查项列表
    QList<CheckItem> runAll();

    // 单项检查
    static bool checkOsVersion();
    static bool checkDiskSpace(const QString &path, qint64 requiredMB = 512);
    static bool checkPort(int port);
    static bool checkAdminPrivileges();
    static QString osVersionString();
    static qint64 availableDiskMB(const QString &path);
};
