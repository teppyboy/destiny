use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileLog {
    pub enabled: bool,
    pub path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Log {
    pub level: String,
    pub file: FileLog,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct List {
    pub enabled: bool,
    pub servers: Vec<u64>,
    pub channels: Vec<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Feature {
    pub enabled: bool,
    pub blacklist: List,
    pub whitelist: List,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct YoutubeDlWorkarounds {
    pub use_pot: bool,
    pub pot_server_port: u16,
    pub use_cookies: bool,
    pub cookies_path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct YoutubeDl {
    pub workarounds: YoutubeDlWorkarounds,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LavalinkNode {
    pub host: String,
    pub port: u16,
    pub password: String,
    pub is_ssl: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Lavalink {
    pub nodes: Vec<LavalinkNode>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MusicPlayer {
    pub enabled: bool,
    pub backend: String,
    pub blacklist: List,
    pub whitelist: List,
    pub lavalink: Lavalink,
    pub ytdl: YoutubeDl,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Features {
    pub music_player: MusicPlayer,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Privileged {
    pub allowed_users: Vec<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct General {
    pub prefix: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub log: Log,
    pub privileged: Privileged,
    pub features: Features,
    pub general: General,
}

impl Config {
    pub fn new() -> Config {
        Config {
            log: Log {
                level: "info".to_string(),
                file: FileLog {
                    enabled: false,
                    path: "destiny-%Y%m%d-%H%M%S.log".to_string(),
                },
            },
            privileged: Privileged {
                allowed_users: vec![],
            },
            features: Features {
                music_player: MusicPlayer {
                    enabled: false,
                    backend: "yt-dlp".to_string(),
                    blacklist: List {
                        enabled: false,
                        servers: vec![],
                        channels: vec![],
                    },
                    whitelist: List {
                        enabled: false,
                        servers: vec![],
                        channels: vec![],
                    },
                    lavalink: Lavalink {
                        nodes: vec![],
                    },
                    ytdl: YoutubeDl {
                        workarounds: YoutubeDlWorkarounds {
                            use_pot: false,
                            pot_server_port: 58553,
                            use_cookies: false,
                            cookies_path: "cookies.txt".to_string(),
                        },
                    },
                },
            },
            general: General {
                prefix: "~".to_string(),
            },
        }
    }
    pub fn save(&self, path: &str) {
        let toml = toml::to_string(&self).unwrap();
        fs::write(path, toml).expect("Failed to write config file");
    }
    pub fn load(path: &str) -> Config {
        let content = fs::read_to_string(path).expect("Failed to read config file");
        let config: Config = toml::from_str(&content.as_str()).unwrap();
        return config;
    }
}
