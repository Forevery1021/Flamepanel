use lettre::{
    Message,
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Tokio1Executor,
};
use lettre::message::header::ContentType;
use crate::core::error::AppError;

#[derive(Debug, Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from: String,
    pub use_tls: bool,
}

impl Default for SmtpConfig {
    fn default() -> Self {
        Self {
            host: "localhost".into(), port: 25,
            username: String::new(), password: String::new(),
            from: "noreply@flamepanel.local".into(), use_tls: false,
        }
    }
}

pub struct EmailNotifier {
    config: SmtpConfig,
}

impl EmailNotifier {
    pub fn new(config: SmtpConfig) -> Self {
        Self { config }
    }

    pub async fn send(&self, to: &str, subject: &str, body: &str) -> Result<(), AppError> {
        let email = Message::builder()
            .from(self.config.from.parse().map_err(|e| AppError::Internal(format!("Invalid from: {}", e)))?)
            .to(to.parse().map_err(|e| AppError::Internal(format!("Invalid to: {}", e)))?)
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body.to_string())
            .map_err(|e| AppError::Internal(format!("Email build: {}", e)))?;

        let creds = Credentials::new(self.config.username.clone(), self.config.password.clone());
        let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&self.config.host)
            .map_err(|e| AppError::Internal(format!("SMTP relay: {}", e)))?
            .port(self.config.port)
            .credentials(creds)
            .build();

        mailer.send(email).await.map_err(|e| AppError::Internal(format!("Send email: {}", e)))?;
        Ok(())
    }
}
