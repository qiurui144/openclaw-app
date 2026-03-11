#include "SystemCheck.h"

#include <QStorageInfo>
#include <QSysInfo>
#include <QTcpSocket>
#include <QVersionNumber>

#ifdef Q_OS_WIN
#  include <windows.h>
#endif
#ifdef Q_OS_LINUX
#  include <sys/utsname.h>
#  include <unistd.h>
#endif

SystemCheck::SystemCheck(QObject *parent) : QObject(parent) {}

QList<CheckItem> SystemCheck::runAll()
{
    QList<CheckItem> items;

    // 1. 操作系统版本
    {
        CheckItem item;
        item.name   = tr("操作系统版本");
        item.passed = checkOsVersion();
        item.detail = osVersionString() +
                      (item.passed ? tr(" ✓ 支持") : tr(" ✗ 需要 Ubuntu 20.04+ 或 Windows 10+"));
        item.required = true;
        items << item;
    }

    // 2. 管理员权限
    {
        CheckItem item;
        item.name   = tr("管理员/root 权限");
        item.passed = checkAdminPrivileges();
        item.detail = item.passed
                    ? tr("已具备管理员权限")
                    : tr("未获取管理员权限（服务注册可能失败）");
        item.required = false;
        items << item;
    }

    // 3. 磁盘空间
    {
        CheckItem item;
        item.name   = tr("可用磁盘空间");
        qint64 avail = availableDiskMB(".");
        item.passed  = avail >= 512;
        item.detail  = tr("%1 MB 可用（需要 ≥ 512 MB）").arg(avail);
        item.required = true;
        items << item;
    }

    // 4. 默认端口 8080
    {
        CheckItem item;
        item.name   = tr("端口 8080 可用");
        item.passed = checkPort(8080);
        item.detail = item.passed
                    ? tr("端口 8080 空闲")
                    : tr("端口 8080 被占用（可在下一步更换端口）");
        item.required = false;
        items << item;
    }

    return items;
}

bool SystemCheck::checkOsVersion()
{
#if defined(Q_OS_LINUX)
    // 检查 kernel 版本 >= 4.x（Ubuntu 20.04 的 kernel 为 5.4）
    struct utsname uts{};
    if (uname(&uts) != 0) return false;
    QString release = QString::fromLatin1(uts.release);
    QVersionNumber ver = QVersionNumber::fromString(release);
    return ver.majorVersion() >= 4;
#elif defined(Q_OS_WIN)
    // Windows 10 = build 10240+
    OSVERSIONINFOEXW osvi{};
    osvi.dwOSVersionInfoSize = sizeof(osvi);
    ULONGLONG mask = VerSetConditionMask(0, VER_MAJORVERSION, VER_GREATER_EQUAL);
    osvi.dwMajorVersion = 10;
    return VerifyVersionInfoW(&osvi, VER_MAJORVERSION, mask) != FALSE;
#else
    return true;
#endif
}

bool SystemCheck::checkDiskSpace(const QString &path, qint64 requiredMB)
{
    return availableDiskMB(path) >= requiredMB;
}

qint64 SystemCheck::availableDiskMB(const QString &path)
{
    QStorageInfo si(path);
    return si.isValid() ? si.bytesAvailable() / (1024 * 1024) : -1;
}

bool SystemCheck::checkPort(int port)
{
    QTcpSocket sock;
    sock.connectToHost("127.0.0.1", static_cast<quint16>(port));
    bool inUse = sock.waitForConnected(500);
    sock.close();
    return !inUse;   // port available == cannot connect
}

bool SystemCheck::checkAdminPrivileges()
{
#if defined(Q_OS_LINUX)
    return geteuid() == 0;
#elif defined(Q_OS_WIN)
    BOOL isAdmin = FALSE;
    PSID adminGroup = nullptr;
    SID_IDENTIFIER_AUTHORITY authority = SECURITY_NT_AUTHORITY;
    if (AllocateAndInitializeSid(&authority, 2,
            SECURITY_BUILTIN_DOMAIN_RID, DOMAIN_ALIAS_RID_ADMINS,
            0, 0, 0, 0, 0, 0, &adminGroup)) {
        CheckTokenMembership(nullptr, adminGroup, &isAdmin);
        FreeSid(adminGroup);
    }
    return isAdmin != FALSE;
#else
    return false;
#endif
}

QString SystemCheck::osVersionString()
{
    return QSysInfo::productType() + " " + QSysInfo::productVersion()
         + " (" + QSysInfo::currentCpuArchitecture() + ")";
}
