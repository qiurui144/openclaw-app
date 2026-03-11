#pragma once
#include <QWizardPage>
#include <QList>
#include <QVBoxLayout>
#include <QLabel>
#include "../core/SystemCheck.h"

class SystemCheckPage : public QWizardPage {
    Q_OBJECT
public:
    explicit SystemCheckPage(QWidget *parent = nullptr);
    void initializePage() override;
    bool isComplete() const override;

private:
    void runChecks();
    void addCheckRow(const CheckItem &item);

    QVBoxLayout *m_listLayout = nullptr;
    QLabel      *m_summaryLabel = nullptr;
    bool         m_allRequired  = false;
};
