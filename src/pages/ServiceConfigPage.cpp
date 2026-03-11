#include "ServiceConfigPage.h"
#include "../DeployWizard.h"

#include <QVBoxLayout>
#include <QFormLayout>
#include <QLabel>
#include <QMessageBox>
#include <QRegularExpressionValidator>

ServiceConfigPage::ServiceConfigPage(QWidget *parent)
    : QWizardPage(parent)
{
    setTitle(tr("服务配置"));
    setSubTitle(tr("配置 OpenClaw 服务的监听端口、域名和管理密码。"));

    auto *root = new QVBoxLayout(this);
    auto *form = new QFormLayout;
    form->setLabelAlignment(Qt::AlignRight);
    form->setRowWrapPolicy(QFormLayout::WrapLongRows);
    form->setVerticalSpacing(10);

    // 端口
    m_portSpin = new QSpinBox(this);
    m_portSpin->setRange(1024, 65535);
    m_portSpin->setValue(8080);
    form->addRow(tr("监听端口："), m_portSpin);

    // 域名（可选）
    m_domainEdit = new QLineEdit(this);
    m_domainEdit->setPlaceholderText(tr("留空则使用 127.0.0.1（本机访问）"));
    form->addRow(tr("绑定域名（可选）："), m_domainEdit);

    // 密码
    m_passEdit = new QLineEdit(this);
    m_passEdit->setEchoMode(QLineEdit::Password);
    m_passEdit->setPlaceholderText(tr("至少 8 位，含字母和数字"));
    form->addRow(tr("管理员密码："), m_passEdit);

    m_pass2Edit = new QLineEdit(this);
    m_pass2Edit->setEchoMode(QLineEdit::Password);
    m_pass2Edit->setPlaceholderText(tr("再次输入密码确认"));
    form->addRow(tr("确认密码："), m_pass2Edit);

    root->addLayout(form);

    auto *note = new QLabel(
        tr("⚠ 请记录管理员密码，部署完成后可通过控制台修改。"), this);
    note->setStyleSheet("color:#e65100;");
    note->setWordWrap(true);
    root->addSpacing(8);
    root->addWidget(note);
    root->addStretch();

    // 注册字段
    registerField(ConfigKey::ServicePort,   m_portSpin,   "value");
    registerField(ConfigKey::DomainName,    m_domainEdit);
    registerField(ConfigKey::AdminPassword + QStringLiteral("*"), m_passEdit);
}

bool ServiceConfigPage::validatePage()
{
    if (m_passEdit->text().length() < 8) {
        QMessageBox::warning(this, tr("密码太短"), tr("管理员密码至少需要 8 位字符。"));
        m_passEdit->setFocus();
        return false;
    }
    if (m_passEdit->text() != m_pass2Edit->text()) {
        QMessageBox::warning(this, tr("密码不一致"), tr("两次输入的密码不相同，请重新输入。"));
        m_pass2Edit->clear();
        m_pass2Edit->setFocus();
        return false;
    }
    // 检查密码强度（至少含一个字母和一个数字）
    QRegularExpression hasLetter("[a-zA-Z]");
    QRegularExpression hasDigit("[0-9]");
    if (!hasLetter.match(m_passEdit->text()).hasMatch() ||
        !hasDigit.match(m_passEdit->text()).hasMatch()) {
        QMessageBox::warning(this, tr("密码强度不足"),
            tr("密码必须同时包含字母和数字。"));
        m_passEdit->setFocus();
        return false;
    }
    return true;
}
