//! Plan 09.2-01: client-ID spike (HARD GATE before the GitHub device-flow driver).
//!
//! Settles decisions D-2 and D-4. Two questions per client ID:
//!   (a) does GitHub's device flow complete (NOT `device_flow_disabled`)?
//!   (b) does the resulting token get accepted by the Dev Tunnels Management API
//!       (`GET /api/v1/tunnels` → 200, not 401)?
//!
//! Tests BOTH the Dev Tunnels GitHub App client ID (`Iv1.e7b89e013f801f03`) and the
//! `gh`-CLI client ID (`178c6fc778ccc68e1d6a`) to decide whether the agent's client ID
//! must also bump (D-4).
//!
//! Run manually (requires a human to authorize in a browser):
//!   VECTOR_SPIKE_RUN=1 cargo test -p vector-tunnels --test client_id_spike \
//!     -- --ignored --nocapture
//!
//! Gated on `VECTOR_SPIKE_RUN=1` (early-return if unset), mirroring the `#[ignore]` +
//! env-gate convention in `live_devtunnel_smoke.rs`. Never prints token VALUES — only
//! presence/length (Pitfall-14 discipline, even though arch-lint scans `src/` not `tests/`).

use std::time::{Duration, SystemTime};

use vector_tunnels::TUNNELS_BASE_URL;

const DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
const TOKEN_URL: &str = "https://github.com/login/oauth/access_token";

/// Dev Tunnels GitHub App client ID (D-2) and the `gh`-CLI client ID (D-4 comparison).
const CLIENT_IDS: [&str; 2] = ["Iv1.e7b89e013f801f03", "178c6fc778ccc68e1d6a"];

/// One row of the verdict table. No token VALUES — only presence (Pitfall-14).
struct Verdict {
    client_id: String,
    device_flow_enabled: bool,
    refresh_token: bool,
    expires_in: Option<u64>,
    list_tunnels_status: u16,
}

/// Parsed `/login/device/code` reply. No secrets in fields read here.
struct DeviceCode {
    code: String,
    user_code: String,
    verification_uri: String,
    interval: Duration,
    expires_in_secs: u64,
}

/// Acquired token outcome — presence flags only, never the token value (Pitfall-14).
struct TokenOutcome {
    access_token: String,
    refresh_present: bool,
    expires_in: Option<u64>,
}

/// Request a device code. `None` => device flow disabled / request failed.
/// (GitHub Apps ignore OAuth scopes; the gh-CLI app expects `read:user`.)
async fn request_device_code(http: &reqwest::Client, client_id: &str) -> Option<DeviceCode> {
    let mut form: Vec<(&str, &str)> = vec![("client_id", client_id)];
    if client_id == "178c6fc778ccc68e1d6a" {
        form.push(("scope", "read:user"));
    }
    let body = match http
        .post(DEVICE_CODE_URL)
        .header(reqwest::header::ACCEPT, "application/json")
        .form(&form)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
    {
        Ok(resp) => resp.text().await.expect("device-code body"),
        Err(e) => {
            println!("device-code request failed for {client_id}: {e}");
            return None;
        }
    };
    let v: serde_json::Value = serde_json::from_str(&body)
        .unwrap_or_else(|e| panic!("non-JSON device-code response for {client_id}: {e}: {body}"));

    // ASSERT device flow is not disabled.
    if v.get("error").and_then(|e| e.as_str()) == Some("device_flow_disabled") {
        println!("DEVICE FLOW DISABLED for {client_id}");
        return None;
    }
    assert_ne!(
        v.get("error").and_then(|e| e.as_str()),
        Some("device_flow_disabled"),
        "device flow must not be disabled for {client_id}"
    );
    if let Some(err) = v.get("error").and_then(|e| e.as_str()) {
        panic!(
            "device-code error for {client_id}: {err}: {}",
            v.get("error_description")
                .and_then(|d| d.as_str())
                .unwrap_or("")
        );
    }

    Some(DeviceCode {
        code: v
            .get("device_code")
            .and_then(|x| x.as_str())
            .unwrap_or_else(|| panic!("missing device_code for {client_id}: {body}"))
            .to_string(),
        user_code: v
            .get("user_code")
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string(),
        // GitHub returns `verification_uri`; fall back to `verification_url` defensively.
        verification_uri: v
            .get("verification_uri")
            .or_else(|| v.get("verification_url"))
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string(),
        interval: Duration::from_secs(
            v.get("interval")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(5)
                .max(1),
        ),
        expires_in_secs: v
            .get("expires_in")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(900),
    })
}

/// Poll the token endpoint until authorized, honoring interval/slow_down.
async fn poll_token(http: &reqwest::Client, client_id: &str, dc: &DeviceCode) -> TokenOutcome {
    let mut interval = dc.interval;
    let deadline = SystemTime::now() + Duration::from_secs(dc.expires_in_secs);
    loop {
        assert!(
            SystemTime::now() < deadline,
            "device code expired for {client_id} before authorization"
        );
        tokio::time::sleep(interval).await;
        let resp = http
            .post(TOKEN_URL)
            .header(reqwest::header::ACCEPT, "application/json")
            .form(&[
                ("client_id", client_id),
                ("device_code", dc.code.as_str()),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .await
            .expect("token poll");
        let status = resp.status();
        let tbody = resp.text().await.expect("token body");
        let tv: serde_json::Value = serde_json::from_str(&tbody).unwrap_or_else(|e| {
            panic!("non-JSON token response ({status}) for {client_id}: {e}: {tbody}")
        });

        if let Some(err) = tv.get("error").and_then(|e| e.as_str()) {
            match err {
                "authorization_pending" => continue,
                "slow_down" => {
                    interval += Duration::from_secs(5);
                    continue;
                }
                "expired_token" => panic!("device code expired for {client_id}"),
                "access_denied" => panic!("authorization denied for {client_id}"),
                other => panic!(
                    "token error for {client_id}: {other}: {}",
                    tv.get("error_description")
                        .and_then(|d| d.as_str())
                        .unwrap_or("")
                ),
            }
        }

        return TokenOutcome {
            access_token: tv
                .get("access_token")
                .and_then(|x| x.as_str())
                .unwrap_or_else(|| panic!("missing access_token for {client_id}: {tbody}"))
                .to_string(),
            refresh_present: tv.get("refresh_token").and_then(|x| x.as_str()).is_some(),
            expires_in: tv.get("expires_in").and_then(serde_json::Value::as_u64),
        };
    }
}

/// Drive one client ID through the full spike: device flow + list_tunnels.
async fn run_one(http: &reqwest::Client, client_id: &str) -> Verdict {
    let Some(dc) = request_device_code(http, client_id).await else {
        return Verdict {
            client_id: client_id.to_string(),
            device_flow_enabled: false,
            refresh_token: false,
            expires_in: None,
            list_tunnels_status: 0,
        };
    };

    // 2. Prompt the human to authorize.
    println!(
        "\n>>> AUTHORIZE: open {} and enter code:  {}",
        dc.verification_uri, dc.user_code
    );
    println!(
        ">>> waiting for sign-in (code expires in {}s)...",
        dc.expires_in_secs
    );

    let token = poll_token(http, client_id, &dc).await;
    println!(
        "token acquired for {client_id} (len={}, refresh_token={}, expires_in={:?})",
        token.access_token.len(),
        token.refresh_present,
        token.expires_in,
    );

    // 4. Call the live Management API with `Authorization: github <token>`.
    let list_url = format!("{TUNNELS_BASE_URL}/api/v1/tunnels");
    let resp = http
        .get(&list_url)
        .header("Authorization", format!("github {}", token.access_token))
        .send()
        .await
        .expect("list_tunnels");
    let list_status = resp.status().as_u16();
    println!("list_tunnels status for {client_id}: {list_status}");

    Verdict {
        client_id: client_id.to_string(),
        device_flow_enabled: true,
        refresh_token: token.refresh_present,
        expires_in: token.expires_in,
        list_tunnels_status: list_status,
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "manual spike — requires VECTOR_SPIKE_RUN=1 + a human to authorize in a browser"]
async fn client_id_spike() {
    if std::env::var("VECTOR_SPIKE_RUN").as_deref() != Ok("1") {
        eprintln!(
            "client_id_spike: VECTOR_SPIKE_RUN unset; skipping (set VECTOR_SPIKE_RUN=1 to run)"
        );
        return;
    }

    let http = reqwest::Client::builder()
        .user_agent(concat!("Vector/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("reqwest client");

    let mut verdicts: Vec<Verdict> = Vec::new();
    for client_id in CLIENT_IDS {
        println!("\n=== Client ID: {client_id} ===");
        verdicts.push(run_one(&http, client_id).await);
    }

    // Verdict table — one row per client ID.
    println!("\n========== VERDICT ==========");
    for v in &verdicts {
        println!(
            "{} | device_flow_enabled={} | refresh_token={} | expires_in={:?} | list_tunnels_status={}",
            v.client_id, v.device_flow_enabled, v.refresh_token, v.expires_in, v.list_tunnels_status
        );
    }
    println!("=============================");
}
