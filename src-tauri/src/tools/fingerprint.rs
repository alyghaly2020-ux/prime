use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct FingerprintProfile {
    pub id: String,
    pub name: String,
    pub user_agent: String,
    pub platform: String,
    pub screen_resolution: (u32, u32),
    pub timezone: String,
    pub language: String,
    pub webgl_vendor: String,
    pub canvas_noise: bool,
    pub webdriver: bool,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct FingerprintManager {
    profiles: RwLock<Vec<FingerprintProfile>>,
    active_profile: RwLock<Option<String>>,
}

#[allow(dead_code)]
impl FingerprintManager {
    pub fn new() -> Self {
        let profiles = Self::default_profiles();
        Self {
            profiles: RwLock::new(profiles),
            active_profile: RwLock::new(None),
        }
    }

    fn default_profiles() -> Vec<FingerprintProfile> {
        vec![
            FingerprintProfile {
                id: "win-chrome-1".into(),
                name: "Windows 11 / Chrome 124".into(),
                user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36".into(),
                platform: "Win32".into(),
                screen_resolution: (1920, 1080),
                timezone: "America/New_York".into(),
                language: "en-US".into(),
                webgl_vendor: "Google Inc. (Intel)".into(),
                canvas_noise: true,
                webdriver: false,
            },
            FingerprintProfile {
                id: "win-firefox-1".into(),
                name: "Windows 11 / Firefox 125".into(),
                user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:125.0) Gecko/20100101 Firefox/125.0".into(),
                platform: "Win32".into(),
                screen_resolution: (1920, 1080),
                timezone: "Europe/London".into(),
                language: "en-GB".into(),
                webgl_vendor: "Mozilla Inc. (NVIDIA)".into(),
                canvas_noise: true,
                webdriver: false,
            },
            FingerprintProfile {
                id: "win-edge-1".into(),
                name: "Windows 10 / Edge 124".into(),
                user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36 Edg/124.0.2478.80".into(),
                platform: "Win32".into(),
                screen_resolution: (1366, 768),
                timezone: "Asia/Tokyo".into(),
                language: "ja-JP".into(),
                webgl_vendor: "Microsoft Corp. (Intel)".into(),
                canvas_noise: true,
                webdriver: false,
            },
            FingerprintProfile {
                id: "mac-chrome-1".into(),
                name: "macOS 14 Sonoma / Chrome 124".into(),
                user_agent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_4) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36".into(),
                platform: "MacIntel".into(),
                screen_resolution: (2560, 1600),
                timezone: "America/Los_Angeles".into(),
                language: "en-US".into(),
                webgl_vendor: "Google Inc. (Apple)".into(),
                canvas_noise: true,
                webdriver: false,
            },
            FingerprintProfile {
                id: "mac-safari-1".into(),
                name: "macOS 14 Sonoma / Safari 17.4".into(),
                user_agent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_4) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Safari/605.1.15".into(),
                platform: "MacIntel".into(),
                screen_resolution: (1512, 982),
                timezone: "America/Los_Angeles".into(),
                language: "en-US".into(),
                webgl_vendor: "Apple Inc. (Apple M2)".into(),
                canvas_noise: false,
                webdriver: false,
            },
            FingerprintProfile {
                id: "mac-firefox-1".into(),
                name: "macOS 13 Ventura / Firefox 125".into(),
                user_agent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 13_6) Gecko/20100101 Firefox/125.0".into(),
                platform: "MacIntel".into(),
                screen_resolution: (1920, 1200),
                timezone: "Europe/Berlin".into(),
                language: "de-DE".into(),
                webgl_vendor: "Mozilla Inc. (Apple M1)".into(),
                canvas_noise: true,
                webdriver: false,
            },
            FingerprintProfile {
                id: "linux-chrome-1".into(),
                name: "Linux (Ubuntu 24) / Chrome 124".into(),
                user_agent: "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36".into(),
                platform: "Linux x86_64".into(),
                screen_resolution: (1920, 1080),
                timezone: "Asia/Shanghai".into(),
                language: "zh-CN".into(),
                webgl_vendor: "Google Inc. (Mesa/X.org)".into(),
                canvas_noise: true,
                webdriver: false,
            },
            FingerprintProfile {
                id: "linux-firefox-1".into(),
                name: "Linux (Ubuntu 24) / Firefox 125".into(),
                user_agent: "Mozilla/5.0 (X11; Linux x86_64; rv:125.0) Gecko/20100101 Firefox/125.0".into(),
                platform: "Linux x86_64".into(),
                screen_resolution: (1366, 768),
                timezone: "Asia/Kolkata".into(),
                language: "en-IN".into(),
                webgl_vendor: "Mozilla Inc. (Mesa/DRI)".into(),
                canvas_noise: true,
                webdriver: false,
            },
            FingerprintProfile {
                id: "win-opera-1".into(),
                name: "Windows 11 / Opera 109".into(),
                user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36 OPR/109.0.0.0".into(),
                platform: "Win32".into(),
                screen_resolution: (1536, 864),
                timezone: "Europe/Moscow".into(),
                language: "ru-RU".into(),
                webgl_vendor: "Google Inc. (NVIDIA)".into(),
                canvas_noise: true,
                webdriver: false,
            },
            FingerprintProfile {
                id: "mac-edge-1".into(),
                name: "macOS 14 Sonoma / Edge 124".into(),
                user_agent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_4) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36 Edg/124.0.2478.80".into(),
                platform: "MacIntel".into(),
                screen_resolution: (1728, 1117),
                timezone: "Australia/Sydney".into(),
                language: "en-AU".into(),
                webgl_vendor: "Microsoft Corp. (Apple M3)".into(),
                canvas_noise: true,
                webdriver: false,
            },
        ]
    }

    pub async fn get_active(&self) -> Option<FingerprintProfile> {
        let active_id = self.active_profile.read().await;
        let id = active_id.as_ref()?;
        let profiles = self.profiles.read().await;
        profiles.iter().find(|p| p.id == id.as_str()).cloned()
    }

    pub async fn set_active(&self, id: &str) -> bool {
        let profiles = self.profiles.read().await;
        if profiles.iter().any(|p| p.id == id) {
            *self.active_profile.write().await = Some(id.into());
            true
        } else {
            false
        }
    }

    pub async fn list(&self) -> Vec<FingerprintProfile> {
        self.profiles.read().await.clone()
    }

    pub async fn random(&self) -> Option<FingerprintProfile> {
        use rand::seq::SliceRandom;
        let profiles = self.profiles.read().await;
        let mut rng = rand::thread_rng();
        profiles.choose(&mut rng).cloned()
    }

    pub async fn add_profile(&self, profile: FingerprintProfile) {
        let mut profiles = self.profiles.write().await;
        profiles.push(profile);
    }

    pub async fn remove_profile(&self, id: &str) -> bool {
        let mut profiles = self.profiles.write().await;
        let len_before = profiles.len();
        profiles.retain(|p| p.id != id);
        if profiles.len() != len_before {
            let mut active = self.active_profile.write().await;
            if active.as_deref() == Some(id) {
                *active = None;
            }
            true
        } else {
            false
        }
    }
}

impl Default for FingerprintManager {
    fn default() -> Self {
        Self::new()
    }
}
