use axum::{
    extract::{ConnectInfo, State},
    http::{HeaderName, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::{
    collections::{HashMap, VecDeque},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Clone, Default)]
pub struct RateLimiter {
    inner: Arc<Mutex<HashMap<(IpAddr, &'static str), VecDeque<Instant>>>>,
}

impl RateLimiter {
    fn allow(
        &self,
        ip: IpAddr,
        bucket: &'static str,
        limit: usize,
        window: Duration,
    ) -> bool {
        let now = Instant::now();
        let mut entries = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        if entries.len() > 10_000 {
            entries.retain(|_, events| {
                events.retain(|event| now.saturating_duration_since(*event) < Duration::from_secs(120));
                !events.is_empty()
            });
        }
        let key = (ip, bucket);
        if !entries.contains_key(&key) && entries.len() >= 50_000 {
            let oldest = entries
                .iter()
                .filter_map(|(key, events)| events.front().map(|event| (*key, *event)))
                .min_by_key(|(_, event)| *event)
                .map(|(key, _)| key);
            if let Some(oldest) = oldest {
                entries.remove(&oldest);
            }
        }

        let events = entries.entry(key).or_default();
        events.retain(|event| now.saturating_duration_since(*event) < window);
        if events.len() >= limit {
            return false;
        }
        events.push_back(now);
        true
    }
}

pub async fn rate_limit(
    State(limiter): State<RateLimiter>,
    request: axum::extract::Request,
    next: Next,
) -> Response {
    // Use the socket peer, not caller-supplied X-Forwarded-For. Configure proxy trust separately.
    let ip = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(address)| address.ip())
        .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));

    let path = request.uri().path();
    let (bucket, limit) = if matches!(
        path,
        "/api/v1/auth/login"
            | "/api/v1/auth/register"
            | "/api/v1/auth/register-learner"
    ) {
        ("auth", 10)
    } else if path == "/api/v1/payments/verify" {
        ("payment_verify", 30)
    } else if path.starts_with("/api/v1/webhooks/") {
        ("webhooks", 600)
    } else {
        ("general", 300)
    };

    if !limiter.allow(ip, bucket, limit, Duration::from_secs(60)) {
        let mut response = (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "error": {
                    "code": "rate_limited",
                    "message": "Too many requests; retry shortly"
                }
            })),
        )
            .into_response();
        response.headers_mut().insert(
            HeaderName::from_static("retry-after"),
            HeaderValue::from_static("60"),
        );
        return response;
    }

    next.run(request).await
}

pub async fn secure_headers(
    request: axum::extract::Request,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    headers.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );
    headers.insert(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(
        HeaderName::from_static("cache-control"),
        HeaderValue::from_static("no-store"),
    );
    headers.insert(
        HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
    );
    headers.insert(
        HeaderName::from_static("content-security-policy"),
        HeaderValue::from_static("default-src 'none'; frame-ancestors 'none'"),
    );

    response
}

#[cfg(test)]
mod tests {
    use super::RateLimiter;
    use std::{
        net::{IpAddr, Ipv4Addr},
        time::Duration,
    };

    #[test]
    fn rate_limit_is_scoped_by_ip_and_bucket() {
        let limiter = RateLimiter::default();
        let first = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1));
        let second = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 2));

        assert!(limiter.allow(first, "auth", 1, Duration::from_secs(60)));
        assert!(!limiter.allow(first, "auth", 1, Duration::from_secs(60)));
        assert!(limiter.allow(first, "general", 1, Duration::from_secs(60)));
        assert!(limiter.allow(second, "auth", 1, Duration::from_secs(60)));
    }
}
