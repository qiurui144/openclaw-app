#pragma once
#include <QWizardPage>
#include <QProgressBar>
#include <QLabel>
#include <QTextEdit>

class DeploymentPage : public QWizardPage {
    Q_OBJECT
public:
    explicit DeploymentPage(QWidget *parent = nullptr);
    void initializePage() override;
    bool isComplete() const override;

private slots:
    void onProgress(int pct, const QString &msg);
    void onDeployed();
    void onDeployFailed(const QString &reason);

private:
    void collectConfigFromWizard();

    QProgressBar *m_progressBar  = nullptr;
    QLabel       *m_statusLabel  = nullptr;
    QTextEdit    *m_logView      = nullptr;
    bool          m_done         = false;
    bool          m_failed       = false;
};
