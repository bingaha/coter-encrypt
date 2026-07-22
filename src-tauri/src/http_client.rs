use std::{
    fs,
    path::PathBuf,
    sync::Mutex,
    time::Duration,
};

use directories::ProjectDirs;
use reqwest::{Client, Proxy};
use serde::{Deserialize, Serialize};
use url::Url;

const CONFIG_FILE_NAME: &str = "http-proxy.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum HttpProxyMode {
    Direct,
    #[default]
    System,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpProxyConfig {
    pub mode: HttpProxyMode,
    #[serde(default)]
    pub url: String,
}

impl Default for HttpProxyConfig {
    fn default() -> Self {
        Self {
            mode: HttpProxyMode::System,
            url: String::new(),
        }
    }
}

pub struct HttpProxyState {
    inner: Mutex<HttpProxyConfig>,
}

impl HttpProxyState {
    pub fn new(config: HttpProxyConfig) -> Self {
        Self {
            inner: Mutex::new(config),
        }
    }

    pub fn get(&self) -> HttpProxyConfig {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn set(&self, config: HttpProxyConfig) {
        *self.inner.lock().unwrap_or_else(|e| e.into_inner()) = config;
    }
}

fn config_path() -> Result<PathBuf, String> {
    let dirs = ProjectDirs::from("com", "coter", "CoterEncrypt")
        .ok_or_else(|| "无法解析应用配置目录".to_string())?;
    Ok(dirs.config_dir().join(CONFIG_FILE_NAME))
}

fn ensure_config_dir(path: &PathBuf) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建配置目录失败: {e}"))?;
    }
    Ok(())
}

pub fn load_http_proxy_config() -> Result<HttpProxyConfig, String> {
    let path = config_path()?;
    if !path.exists() {
        let config = HttpProxyConfig::default();
        save_http_proxy_config_to_disk(&config)?;
        return Ok(config);
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("读取代理配置失败: {e}"))?;
    let config: HttpProxyConfig =
        serde_json::from_str(&content).map_err(|e| format!("解析代理配置失败: {e}"))?;
    Ok(normalize_config(config)?)
}

pub fn save_http_proxy_config_to_disk(config: &HttpProxyConfig) -> Result<(), String> {
    let path = config_path()?;
    ensure_config_dir(&path)?;
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| format!("序列化代理配置失败: {e}"))?;
    fs::write(&path, content).map_err(|e| format!("写入代理配置失败: {e}"))?;
    Ok(())
}

pub fn normalize_config(mut config: HttpProxyConfig) -> Result<HttpProxyConfig, String> {
    config.url = config.url.trim().to_string();
    match config.mode {
        HttpProxyMode::Custom => {
            if config.url.is_empty() {
                return Err("指定代理模式下请填写代理地址".to_string());
            }
            let parsed = Url::parse(&config.url).map_err(|e| format!("代理地址无效: {e}"))?;
            match parsed.scheme() {
                "http" | "https" => {}
                other => {
                    return Err(format!(
                        "仅支持 HTTP/HTTPS 代理，当前为: {other}"
                    ));
                }
            }
            if parsed.host_str().is_none() {
                return Err("代理地址缺少主机名".to_string());
            }
        }
        HttpProxyMode::Direct | HttpProxyMode::System => {
            // URL 可为空，保留但不强制使用
        }
    }
    Ok(config)
}

pub fn build_http_client(timeout: Duration, config: &HttpProxyConfig) -> Result<Client, String> {
    let mut builder = Client::builder().timeout(timeout);
    match config.mode {
        HttpProxyMode::Direct => {
            builder = builder.no_proxy();
        }
        HttpProxyMode::System => {
            // 跟随 HTTP_PROXY / HTTPS_PROXY / NO_PROXY
        }
        HttpProxyMode::Custom => {
            let proxy = Proxy::all(config.url.as_str())
                .map_err(|e| format!("创建代理失败: {e}"))?;
            builder = builder.proxy(proxy);
        }
    }
    builder
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))
}

pub fn create_state() -> HttpProxyState {
    let config = load_http_proxy_config().unwrap_or_default();
    HttpProxyState::new(config)
}
