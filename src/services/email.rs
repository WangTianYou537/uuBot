use lettre::message::Mailbox;
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use rand::RngExt;

use crate::error::AppError;
use crate::services::settings::SmtpSettings;

/// Generate a random 6-digit numeric verification code.
pub fn generate_code() -> String {
    let n: u32 = rand::rng().random_range(0..1_000_000);
    format!("{n:06}")
}

/// Send a plaintext email using the configured SMTP relay.
pub async fn send_email(
    cfg: &SmtpSettings,
    to: &str,
    subject: &str,
    body: String,
) -> Result<(), AppError> {
    if !cfg.enabled {
        return Err(AppError::BadRequest("SMTP 未启用,无法发送邮件".into()));
    }
    if cfg.host.is_empty() || cfg.from_email.is_empty() {
        return Err(AppError::BadRequest("SMTP 配置不完整".into()));
    }

    let from: Mailbox = format!("{} <{}>", cfg.from_name, cfg.from_email)
        .parse()
        .map_err(|e| AppError::BadRequest(format!("发件人地址无效: {e}")))?;
    let to_mbox: Mailbox = to
        .parse()
        .map_err(|e| AppError::BadRequest(format!("收件人地址无效: {e}")))?;

    let email = Message::builder()
        .from(from)
        .to(to_mbox)
        .subject(subject)
        .header(ContentType::TEXT_PLAIN)
        .body(body)
        .map_err(|e| AppError::Internal(format!("构建邮件失败: {e}")))?;

    let creds = Credentials::new(cfg.username.clone(), cfg.password.clone());
    let builder = if cfg.use_implicit_tls {
        AsyncSmtpTransport::<Tokio1Executor>::relay(&cfg.host)
    } else {
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&cfg.host)
    }
    .map_err(|e| AppError::Internal(format!("SMTP 连接配置失败: {e}")))?;

    let mailer = builder.port(cfg.port).credentials(creds).build();

    mailer
        .send(email)
        .await
        .map_err(|e| AppError::Internal(format!("邮件发送失败: {e}")))?;
    Ok(())
}

/// Send a verification code email for the given purpose label (shown to the user).
pub async fn send_code_email(
    cfg: &SmtpSettings,
    to: &str,
    code: &str,
    purpose_label: &str,
) -> Result<(), AppError> {
    let subject = format!("【uuBot】{purpose_label}验证码");
    let body = format!(
        "您的{purpose_label}验证码是: {code}\n\n该验证码 10 分钟内有效,请勿泄露给他人。\n如非本人操作,请忽略此邮件。"
    );
    send_email(cfg, to, &subject, body).await
}
