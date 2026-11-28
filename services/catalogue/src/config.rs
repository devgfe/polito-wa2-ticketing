use anyhow::Context;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub database_url: String,
    pub page_size_default: i64,
    pub page_size_max: i64,
    pub cors_origins: Vec<String>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        // Carica .env se c'è; se manca (es. in Docker) va bene lo stesso.
        // Una variabile già impostata nella shell vince sul file.
        dotenvy::dotenv().ok();

        let config = Self {
            port: env_or("PORT", "3001")
                .parse()
                .context("PORT must be a port number")?,
            database_url: std::env::var("DATABASE_URL").context("DATABASE_URL is required")?,
            page_size_default: env_or("PAGE_SIZE_DEFAULT", "20")
                .parse()
                .context("PAGE_SIZE_DEFAULT must be a number")?,
            page_size_max: env_or("PAGE_SIZE_MAX", "200")
                .parse()
                .context("PAGE_SIZE_MAX must be a number")?,
            cors_origins: env_or("CORS_ORIGINS", "")
                .split(',')
                .map(str::trim)
                .filter(|o| !o.is_empty())
                .map(String::from)
                .collect(),
        };

        anyhow::ensure!(
            1 <= config.page_size_default && config.page_size_default <= config.page_size_max,
            "PAGE_SIZE_DEFAULT must be between 1 and PAGE_SIZE_MAX"
        );
        Ok(config)
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}
