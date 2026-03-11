#pragma once
#include <QWizardPage>

class FinishPage : public QWizardPage {
    Q_OBJECT
public:
    explicit FinishPage(QWidget *parent = nullptr);
    void initializePage() override;

private:
    void openDashboard();
};
