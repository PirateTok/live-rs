use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

use crate::errors::TikTokLiveError;
use crate::http::api::FetchParams;
use crate::http::sigi::{normalize_username, scrape_profile, SigiProfile};
use crate::http::ttwid::{fetch_ttwid, TtwidRequest};
use crate::http::ua::random_ua;
use crate::structs::config::{Endpoints, PROFILE_CACHE_TTL, PROFILE_SCRAPE_TIMEOUT, PROFILE_TTWID_TIMEOUT, TTWID_FETCH_ATTEMPTS, TTWID_RETRY_DELAY};

enum CacheEntry {
    Profile(SigiProfile, Instant),
    Error(TikTokLiveError, Instant),
}

#[derive(Clone, Debug)]
pub enum CacheLookup {
    Hit(SigiProfile),
    Miss,
}

struct CacheInner {
    entries: HashMap<String, CacheEntry>,
    ttwid: Option<String>,
    ttl: std::time::Duration,
    proxy: Option<String>,
    user_agent: String,
    cookies: Option<String>,
    endpoints: Endpoints,
}

#[derive(Clone)]
pub struct ProfileCache {
    inner: Arc<Mutex<CacheInner>>,
}

impl ProfileCache {
    pub fn new() -> Self {
        Self::with_ttl(PROFILE_CACHE_TTL)
    }

    pub fn with_ttl(ttl: std::time::Duration) -> Self {
        Self {
            inner: Arc::new(Mutex::new(CacheInner {
                entries: HashMap::new(),
                ttwid: None,
                ttl,
                proxy: None,
                user_agent: random_ua().to_string(),
                cookies: None,
                endpoints: Endpoints::tiktok(),
            })),
        }
    }

    pub fn endpoints(self, endpoints: Endpoints) -> Self {
        self.lock().endpoints = endpoints;
        self
    }

    pub fn proxy(self, url: impl Into<String>) -> Self {
        self.lock().proxy = Some(url.into());
        self
    }

    pub fn user_agent(self, ua: impl Into<String>) -> Self {
        self.lock().user_agent = ua.into();
        self
    }

    pub fn cookies(self, cookies: impl Into<String>) -> Self {
        self.lock().cookies = Some(cookies.into());
        self
    }

    pub async fn fetch(&self, username: &str) -> Result<SigiProfile, TikTokLiveError> {
        let key = normalize_key(username);
        match self.fresh_entry(&key) {
            FreshEntry::Profile(profile) => return Ok(profile),
            FreshEntry::Error(err) => return Err(err),
            FreshEntry::Absent => {}
        }

        let ttwid = self.ensure_ttwid().await?;
        let (proxy, user_agent, cookies, endpoints) = {
            let inner = self.lock();
            (inner.proxy.clone(), inner.user_agent.clone(), inner.cookies.clone(), inner.endpoints.clone())
        };
        let params = FetchParams {
            timeout: PROFILE_SCRAPE_TIMEOUT,
            cookies: cookies.as_deref(),
            user_agent: Some(&user_agent),
            proxy: proxy.as_deref(),
            language: None,
            region: None,
            endpoints: &endpoints,
        };

        match scrape_profile(&key, &ttwid, &params).await {
            Ok(profile) => {
                self.lock().entries.insert(key, CacheEntry::Profile(profile.clone(), Instant::now()));
                Ok(profile)
            }
            Err(err) => {
                if is_negative_cacheable(&err) {
                    self.lock().entries.insert(key, CacheEntry::Error(clone_profile_error(&err), Instant::now()));
                }
                tracing::warn!(error = %err, "profile fetch failed");
                Err(err)
            }
        }
    }

    pub fn cached(&self, username: &str) -> CacheLookup {
        match self.fresh_entry(&normalize_key(username)) {
            FreshEntry::Profile(profile) => CacheLookup::Hit(profile),
            FreshEntry::Error(err) => {
                tracing::warn!(error = %err, "cached profile lookup hit a negative entry");
                CacheLookup::Miss
            }
            FreshEntry::Absent => CacheLookup::Miss,
        }
    }

    pub fn invalidate(&self, username: &str) {
        let key = normalize_key(username);
        self.lock().entries.remove(&key);
    }

    pub fn invalidate_all(&self) {
        self.lock().entries.clear();
    }

    fn fresh_entry(&self, key: &str) -> FreshEntry {
        let inner = self.lock();
        let ttl = inner.ttl;
        let Some(entry) = inner.entries.get(key) else {
            return FreshEntry::Absent;
        };
        match entry {
            CacheEntry::Profile(profile, ts) if ts.elapsed() < ttl => FreshEntry::Profile(profile.clone()),
            CacheEntry::Error(err, ts) if ts.elapsed() < ttl => FreshEntry::Error(clone_profile_error(err)),
            CacheEntry::Profile(..) | CacheEntry::Error(..) => FreshEntry::Absent,
        }
    }

    async fn ensure_ttwid(&self) -> Result<String, TikTokLiveError> {
        let (cached, proxy, user_agent, web) = {
            let inner = self.lock();
            (inner.ttwid.clone(), inner.proxy.clone(), inner.user_agent.clone(), inner.endpoints.web.clone())
        };
        for ttwid in cached.iter() {
            return Ok(ttwid.clone());
        }

        let request = TtwidRequest {
            url: &web,
            timeout: PROFILE_TTWID_TIMEOUT,
            user_agent: &user_agent,
            proxy: proxy.as_deref(),
            attempts: TTWID_FETCH_ATTEMPTS,
            retry_delay: TTWID_RETRY_DELAY,
        };
        let ttwid = fetch_ttwid(&request).await?;
        self.lock().ttwid = Some(ttwid.clone());
        Ok(ttwid)
    }

    fn lock(&self) -> MutexGuard<'_, CacheInner> {
        match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                tracing::warn!(error = %poisoned, "profile cache mutex poisoned, recovering");
                poisoned.into_inner()
            }
        }
    }
}

enum FreshEntry {
    Profile(SigiProfile),
    Error(TikTokLiveError),
    Absent,
}

fn normalize_key(username: &str) -> String {
    normalize_username(username)
}

fn is_negative_cacheable(err: &TikTokLiveError) -> bool {
    matches!(err, TikTokLiveError::ProfilePrivate(_) | TikTokLiveError::ProfileNotFound(_) | TikTokLiveError::ProfileError(_))
}

fn clone_profile_error(err: &TikTokLiveError) -> TikTokLiveError {
    match err {
        TikTokLiveError::ProfilePrivate(u) => TikTokLiveError::ProfilePrivate(u.clone()),
        TikTokLiveError::ProfileNotFound(u) => TikTokLiveError::ProfileNotFound(u.clone()),
        TikTokLiveError::ProfileError(c) => TikTokLiveError::ProfileError(*c),
        other => TikTokLiveError::invalid(format!("{other}")),
    }
}
