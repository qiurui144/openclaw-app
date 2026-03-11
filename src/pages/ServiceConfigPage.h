#pragma once
#include <QWizardPage>
#include <QLineEdit>
#include <QSpinBox>

class ServiceConfigPage : public QWizardPage {
    Q_OBJECT
public:
    explicit ServiceConfigPage(QWidget *parent = nullptr);
    bool validatePage() override;

private:
    QSpinBox  *m_portSpin    = nullptr;
    QLineEdit *m_domainEdit  = nullptr;
    QLineEdit *m_passEdit    = nullptr;
    QLineEdit *m_pass2Edit   = nullptr;
};
