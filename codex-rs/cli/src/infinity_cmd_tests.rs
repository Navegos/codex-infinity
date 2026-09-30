use super::*;
use codex_http_client::HttpClientFactory;
use codex_http_client::OutboundProxyPolicy;
use pretty_assertions::assert_eq;
use serde_json::json;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::body_json;
use wiremock::matchers::header;
use wiremock::matchers::method;
use wiremock::matchers::path;

fn test_client(origin: &str) -> HttpClient {
    HttpClientBuilder::new()
        .without_redirects()
        .without_request_logging()
        .build_respecting_outbound_proxy_policy(
            &HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault),
            origin,
            ClientRouteClass::Api,
        )
        .unwrap()
}

#[test]
fn setup_instructions_snapshot() {
    insta::assert_snapshot!(SETUP, @"
    Sign in at https://codex-infinity.com/account and copy your API key.
    Set CODEX_INFINITY_API_KEY in your shell, then run `codex infinity connect`.
    Codex automatically enables Infinity MCP tools when the key is present.
    ");
}

#[test]
fn sync_requires_explicit_provider_selection() {
    assert!(InfinityCli::try_parse_from(["infinity", "sync-keys"]).is_err());
    assert!(
        InfinityCli::try_parse_from([
            "infinity",
            "sync-keys",
            "--provider",
            "openai",
            "--provider",
            "fal"
        ])
        .is_ok()
    );
}

#[test]
fn only_selected_keys_are_read_and_uploaded() {
    let payload = selected_keys(
        &[Provider::Openai, Provider::Fal, Provider::Openai],
        |name| match name {
            "OPENAI_API_KEY" => Some(" selected-openai ".to_string()),
            "FAL_API_KEY" => Some("selected-fal".to_string()),
            _ => panic!("unselected variable was read: {name}"),
        },
    )
    .unwrap();
    assert_eq!(
        payload,
        BTreeMap::from([
            ("openai_api_key".to_string(), "selected-openai".to_string()),
            ("fal_api_key".to_string(), "selected-fal".to_string()),
        ])
    );
}

#[test]
fn provider_key_aliases_use_primary_values_then_fallbacks() {
    for primary_present in [true, false] {
        let payload = selected_keys(&[Provider::Google, Provider::Fal], |name| match name {
            "GOOGLE_AI_KEY" | "FAL_API_KEY" if primary_present => Some("primary".into()),
            "GOOGLE_AI_KEY" | "FAL_API_KEY" => Some(" ".into()),
            "GOOGLE_API_KEY" | "FAL_KEY" => Some("fallback".into()),
            _ => panic!("unexpected environment read: {name}"),
        })
        .unwrap();
        let expected = if primary_present {
            "primary"
        } else {
            "fallback"
        };
        assert_eq!(
            payload,
            BTreeMap::from([
                ("google_ai_key".to_string(), expected.to_string()),
                ("fal_api_key".to_string(), expected.to_string()),
            ])
        );
    }
}

#[test]
fn missing_key_prevents_partial_upload() {
    let error = selected_keys(&[Provider::Openai, Provider::Fal], |name| {
        (name == "OPENAI_API_KEY").then(|| "private-key".to_string())
    })
    .unwrap_err();
    assert_eq!(
        error.to_string(),
        "FAL_API_KEY or FAL_KEY is not set; no provider keys were uploaded"
    );
}

#[tokio::test]
async fn sync_sends_authenticated_selected_payload() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/account/provider-keys"))
        .and(header("authorization", "Bearer infinity-test-key"))
        .and(body_json(json!({"openai_api_key": "provider-test-key"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"openai_api_key": "masked"})))
        .expect(1)
        .mount(&server)
        .await;
    upload_keys(
        &test_client(&server.uri()),
        &server.uri(),
        "infinity-test-key",
        &BTreeMap::from([(
            "openai_api_key".to_string(),
            "provider-test-key".to_string(),
        )]),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn auth_check_rejects_unauthorized_without_echoing_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/account/provider-keys"))
        .and(header("authorization", "Bearer invalid-key"))
        .respond_with(ResponseTemplate::new(401).set_body_string("secret-from-server"))
        .expect(1)
        .mount(&server)
        .await;
    let error = verify_account(&test_client(&server.uri()), &server.uri(), "invalid-key")
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Infinity authentication check failed (HTTP 401)"
    );
}

#[tokio::test]
async fn sync_rejects_redirect_without_forwarding_credentials() {
    let server = MockServer::start().await;
    let destination = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(307).insert_header("Location", destination.uri()))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&destination)
        .await;
    let error = upload_keys(
        &test_client(&server.uri()),
        &server.uri(),
        "infinity-test-key",
        &BTreeMap::from([(
            "openai_api_key".to_string(),
            "provider-test-key".to_string(),
        )]),
    )
    .await
    .unwrap_err();
    assert_eq!(error.to_string(), "Infinity key sync failed (HTTP 307)");
}
