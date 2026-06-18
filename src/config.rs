use serde::Deserialize;

/// Bootstrap configuration loaded from environment variables (optionally via a
/// `.env` file). Runtime-tunable settings (SMTP, OAuth provider, dictionary)
/// live in the database `settings` table and are edited from the admin panel.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// SeaORM connection string. Examples:
    ///   sqlite://data.db?mode=rwc
    ///   mysql://user:pass@localhost:3306/uubot
    ///   postgres://user:pass@localhost:5432/uubot
    pub database_url: String,
    /// Address to bind the HTTP server, e.g. 0.0.0.0:8080
    pub bind_addr: String,
    /// Secret used to sign JWT session tokens.
    pub jwt_secret: String,
    /// Public base URL of the site, used to build OAuth redirect_uri.
    pub public_base_url: String,
    /// Seed admin account (created on first run if no admin exists).
    pub admin_username: String,
    pub admin_password: String,
    /// Whether the session cookie should be marked Secure (set true behind HTTPS).
    pub cookie_secure: bool,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        // Load .env if present; ignore if missing.
        let _ = dotenvy::dotenv();

        fn var(key: &str, default: &str) -> String {
            std::env::var(key).unwrap_or_else(|_| default.to_string())
        }

        Ok(Config {
            database_url: var("DATABASE_URL", "sqlite://data.db?mode=rwc"),
            bind_addr: var("BIND_ADDR", "0.0.0.0:8080"),
            jwt_secret: var("JWT_SECRET", "change-me-in-production-please"),
            public_base_url: var("PUBLIC_BASE_URL", "http://localhost:8080"),
            admin_username: var("ADMIN_USERNAME", "admin"),
            admin_password: var("ADMIN_PASSWORD", "admin123"),
            cookie_secure: var("COOKIE_SECURE", "false") == "true",
        })
    }
}
