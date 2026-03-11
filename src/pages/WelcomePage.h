#pragma once
#include <QWizardPage>
#include <QLabel>
#include <QVBoxLayout>

class WelcomePage : public QWizardPage {
    Q_OBJECT
public:
    explicit WelcomePage(QWidget *parent = nullptr);
    void initializePage() override;
};
