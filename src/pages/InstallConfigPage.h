#pragma once
#include <QWizardPage>
#include <QLineEdit>
#include <QCheckBox>

class InstallConfigPage : public QWizardPage {
    Q_OBJECT
public:
    explicit InstallConfigPage(QWidget *parent = nullptr);
    void initializePage() override;
    bool validatePage() override;

private:
    QLineEdit *m_pathEdit        = nullptr;
    QCheckBox *m_installService  = nullptr;
    QCheckBox *m_startOnBoot     = nullptr;
};
