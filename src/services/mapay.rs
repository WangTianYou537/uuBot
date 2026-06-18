use serde::Deserialize;

use crate::error::AppError;
use crate::services::settings::OAuthSettings;

/// Response of the `act=login` step: a redirect URL (and a QR-code URL for wx).
#[derive(Debug, Deserialize)]
struct LoginResp {
    code: i64,
    #[serde(default)]
    msg: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    qrcode: String,
}

/// Result of starting a login: where to send the user.
#[derive(Debug)]
pub struct LoginRedirect {
    pub url: String,
    pub qrcode: String,
}

/// Response of the `act=callback` step: the resolved third-party identity.
#[derive(Debug, Deserialize)]
struct CallbackResp {
    code: i64,
    #[serde(default)]
    msg: String,
    #[serde(default)]
    social_uid: String,
    #[serde(default)]
    nickname: String,
    #[serde(default)]
    faceimg: String,
}

/// The identity we persist as a user.
#[derive(Debug)]
pub struct SocialIdentity {
    pub social_uid: String,
    pub nickname: String,
    pub avatar: String,
}

fn ensure_configured(cfg: &OAuthSettings) -> Result<(), AppError> {
    if cfg.appid.is_empty() || cfg.appkey.is_empty() {
        return Err(AppError::BadRequest(
            "微信登录尚未配置,请联系管理员在后台填写 appid/appkey".into(),
        ));
    }
    Ok(())
}

/// Step 1: ask the provider for a login redirect URL for the given `login_type`
/// (e.g. "wx"). `redirect_uri` is where the provider sends the user back.
pub async fn start_login(
    http: &reqwest::Client,
    cfg: &OAuthSettings,
    login_type: &str,
    redirect_uri: &str,
) -> Result<LoginRedirect, AppError> {
    ensure_configured(cfg)?;
    let resp: LoginResp = http
        .get(&cfg.base_url)
        .query(&[
            ("act", "login"),
            ("appid", cfg.appid.as_str()),
            ("appkey", cfg.appkey.as_str()),
            ("type", login_type),
            ("redirect_uri", redirect_uri),
        ])
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("登录服务请求失败: {e}")))?
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("登录服务返回解析失败: {e}")))?;

    if resp.code != 0 {
        return Err(AppError::BadRequest(format!(
            "登录服务返回错误: {}",
            resp.msg
        )));
    }
    Ok(LoginRedirect {
        url: resp.url,
        qrcode: resp.qrcode,
    })
}

/// Step 4: exchange the authorization `code` for the third-party identity.
pub async fn exchange_code(
    http: &reqwest::Client,
    cfg: &OAuthSettings,
    login_type: &str,
    code: &str,
) -> Result<SocialIdentity, AppError> {
    ensure_configured(cfg)?;
    let resp: CallbackResp = http
        .get(&cfg.base_url)
        .query(&[
            ("act", "callback"),
            ("appid", cfg.appid.as_str()),
            ("appkey", cfg.appkey.as_str()),
            ("type", login_type),
            ("code", code),
        ])
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("登录回调请求失败: {e}")))?
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("登录回调返回解析失败: {e}")))?;

    if resp.code != 0 {
        return Err(AppError::BadRequest(format!(
            "登录回调失败: {}",
            resp.msg
        )));
    }
    if resp.social_uid.is_empty() {
        return Err(AppError::BadRequest("登录回调未返回用户标识".into()));
    }
    Ok(SocialIdentity {
        social_uid: resp.social_uid,
        nickname: if resp.nickname.is_empty() {
            "微信用户".into()
        } else {
            resp.nickname
        },
        avatar: resp.faceimg,
    })
}
