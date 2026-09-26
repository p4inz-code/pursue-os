//! Integration and security test suite for `pursue-browser`.
//!
//! Validates:
//! - Test 1: Direct mode navigation, evidence capture, case attachment, and audit chaining.
//! - Test 2: Tor mode navigation with .onion addressing and explicit routing verification.
//! - Test 3: Tor unavailable -> strict FAIL CLOSED without falling back to direct network.
//! - Test 4: Direct mode never silently routes through Tor.
//! - Test 5: Dangerous URL schemes (`file://`, `data:`, `javascript:`, `about:`) strictly rejected.
//! - Test 6: Bounded response body reading and truncation flag enforcement.
//! - Test 7: Profile directory isolation: Case A and Case B have completely separate paths.
//! - Test 8: Case isolation: Evidence in Case A cannot be retrieved from Case B.
//! - Test 9: IPC lifecycle over `BrowserHandler`: create, get, navigate, capture, terminate.
//! - Test 10: Corrupted web evidence detection fails closed on case load.
//! - Test 11: Sensitive headers (Authorization, Cookie) scrubbed from provenance and display.
//! - Test 12: Terminated session rejects navigation requests.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use pursue_browser::{
    BrowserEngine, BrowserHandler, BrowserService, BrowserSession, BrowserSessionId,
    MockBrowserEngine, MockResponse, NavigationRequest, NavigationResult, RoutingMode,
    ValidatedUrl, WebArtifactKind, WebEvidenceCapturer, scrub_sensitive_headers,
};
use pursue_case::{CaseId, CaseStore, FileCaseStore};
use pursue_core::Error;
use pursue_evidence::EvidenceStore;
use pursue_runtime::ipc::dispatch::Handler;
use pursue_runtime::ipc::protocol::MethodName;
use serde_json::json;

fn temp_dir(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("pursue_browser_test_{name}_{nonce}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn test_1_direct_mode_navigation_and_evidence_capture_flow() {
    let root = temp_dir("test_1");
    let mut store = FileCaseStore::open(&root).unwrap();

    let case_id = CaseId::new("case-001").unwrap();
    let actor = "analyst-1";
    store
        .create_case(case_id.clone(), "OSINT Investigation 1", actor)
        .unwrap();

    let session_id = BrowserSessionId::new("session-direct").unwrap();
    let session = BrowserSession::new(
        session_id,
        case_id.clone(),
        actor,
        RoutingMode::Direct,
        &root,
    )
    .unwrap();

    let engine = MockBrowserEngine::new();
    let url = ValidatedUrl::parse("http://target-intel.com/report.html").unwrap();
    let html_body = b"<html><body>Critical Target Intelligence Report</body></html>".to_vec();

    let mut headers = BTreeMap::new();
    headers.insert("content-type".into(), "text/html".into());
    headers.insert("server".into(), "nginx/1.24".into());

    engine.register(
        "http://target-intel.com",
        MockResponse::Success {
            status_code: 200,
            headers: headers.clone(),
            body: html_body.clone(),
            final_url: None,
        },
    );

    let req = NavigationRequest::new(url);
    let nav_result = engine.navigate(&session, &req).unwrap();

    assert_eq!(nav_result.status_code(), 200);
    assert_eq!(nav_result.routing_mode(), RoutingMode::Direct);
    assert_eq!(nav_result.body(), html_body.as_slice());

    // Capture Page Content as evidence
    let captured = WebEvidenceCapturer::capture_navigation_artifact(
        &mut store,
        &session,
        &nav_result,
        WebArtifactKind::PageContent,
        None,
    )
    .unwrap();

    // Verify stored in case evidence repository
    let ev_store = store.open_evidence_store(&case_id).unwrap();
    assert!(ev_store.contains(&captured.address));
    let blob = ev_store.get(&captured.address).unwrap();
    assert_eq!(blob, html_body.as_slice());

    // Verify attached to case manifest
    let loaded_case = store.load_case(&case_id).unwrap();
    assert!(
        loaded_case
            .evidence_addresses()
            .any(|a| a == &captured.address)
    );

    // Verify audit chain
    assert!(loaded_case.audit_log().verify().is_ok());
    let last_event = loaded_case.audit_log().entries().last().unwrap();
    assert_eq!(last_event.action, "case.evidence.attached");

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_2_tor_mode_with_onion_addressing() {
    let root = temp_dir("test_2");
    let mut store = FileCaseStore::open(&root).unwrap();

    let case_id = CaseId::new("case-002").unwrap();
    store
        .create_case(case_id.clone(), "Darkweb Investigation", "analyst")
        .unwrap();

    let session_id = BrowserSessionId::new("session-tor").unwrap();
    let session =
        BrowserSession::new(session_id, case_id, "analyst", RoutingMode::Tor, &root).unwrap();

    let engine = MockBrowserEngine::new();
    let onion_url = ValidatedUrl::parse(
        "http://duckduckgogg42xjoc72x3sjasowoarfbgcmvfimaftt6twagswzczad.onion/search?q=cyber",
    )
    .unwrap();
    assert!(onion_url.is_onion());

    engine.register(
        "http://duckduckgogg42xjoc72x3sjasowoarfbgcmvfimaftt6twagswzczad.onion",
        MockResponse::Success {
            status_code: 200,
            headers: BTreeMap::new(),
            body: b"Darkweb Search Results".to_vec(),
            final_url: None,
        },
    );

    let req = NavigationRequest::new(onion_url);
    let nav_result = engine.navigate(&session, &req).unwrap();

    assert_eq!(nav_result.status_code(), 200);
    assert_eq!(nav_result.routing_mode(), RoutingMode::Tor);
    assert!(nav_result.request_url().is_onion());

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_3_tor_unavailable_fails_closed_without_direct_fallback() {
    let root = temp_dir("test_3");
    let case_id = CaseId::new("case-003").unwrap();
    let session = BrowserSession::new(
        BrowserSessionId::new("sess-fail-closed").unwrap(),
        case_id,
        "analyst",
        RoutingMode::Tor,
        &root,
    )
    .unwrap();

    let engine = MockBrowserEngine::new();
    // Simulate Tor daemon offline / SOCKS proxy down
    engine.set_tor_available(false);

    let url = ValidatedUrl::parse("http://example.com/sensitive-target").unwrap();
    let req = NavigationRequest::new(url);

    // Must FAIL CLOSED
    let err = engine.navigate(&session, &req).unwrap_err();
    assert!(err.to_string().contains(
        "Tor proxy is unavailable; failing closed without fallback to direct networking"
    ));

    // Verify ZERO requests were recorded
    assert_eq!(engine.recorded_calls().len(), 0);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_4_direct_mode_never_routes_through_tor_and_rejects_onion() {
    let root = temp_dir("test_4");
    let session = BrowserSession::new(
        BrowserSessionId::new("sess-direct-only").unwrap(),
        CaseId::new("case-004").unwrap(),
        "analyst",
        RoutingMode::Direct,
        &root,
    )
    .unwrap();

    let engine = MockBrowserEngine::new();

    // 1. Regular URL routes directly
    let direct_url = ValidatedUrl::parse("https://example.com/clearweb").unwrap();
    let res = engine
        .navigate(&session, &NavigationRequest::new(direct_url))
        .unwrap();
    assert_eq!(res.routing_mode(), RoutingMode::Direct);

    // 2. Attempting to navigate an .onion in Direct mode is rejected
    let onion_url = ValidatedUrl::parse("http://hiddenxyz1234567.onion/").unwrap();
    let onion_err = engine
        .navigate(&session, &NavigationRequest::new(onion_url))
        .unwrap_err();
    assert!(onion_err.to_string().contains("Tor routing required"));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_5_dangerous_schemes_strictly_rejected() {
    assert!(ValidatedUrl::parse("file:///etc/shadow").is_err());
    assert!(ValidatedUrl::parse("data:text/html,<script>evil()</script>").is_err());
    assert!(ValidatedUrl::parse("javascript:alert(document.cookie)").is_err());
    assert!(ValidatedUrl::parse("about:config").is_err());
    assert!(ValidatedUrl::parse("chrome://flags").is_err());
    assert!(ValidatedUrl::parse("ftp://malicious.org/dump").is_err());
}

#[test]
fn test_6_bounded_response_body_reading_and_truncation() {
    let root = temp_dir("test_6");
    let session = BrowserSession::new(
        BrowserSessionId::new("sess-trunc").unwrap(),
        CaseId::new("case-006").unwrap(),
        "analyst",
        RoutingMode::Direct,
        &root,
    )
    .unwrap();

    let engine = MockBrowserEngine::new();
    let oversized_payload = vec![b'Z'; 5000];
    engine.register(
        "http://oversized.org",
        MockResponse::Success {
            status_code: 200,
            headers: BTreeMap::new(),
            body: oversized_payload,
            final_url: None,
        },
    );

    let url = ValidatedUrl::parse("http://oversized.org/data").unwrap();
    let req = NavigationRequest::new(url)
        .with_max_response_bytes(1024)
        .unwrap();

    let res = engine.navigate(&session, &req).unwrap();
    assert_eq!(res.body().len(), 1024);
    assert!(res.is_truncated());

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_7_profile_directory_isolation_between_cases() {
    let root = temp_dir("test_7");
    let sess1 = BrowserSession::new(
        BrowserSessionId::new("sess-alpha").unwrap(),
        CaseId::new("case-alpha").unwrap(),
        "analyst",
        RoutingMode::Direct,
        &root,
    )
    .unwrap();

    let sess2 = BrowserSession::new(
        BrowserSessionId::new("sess-beta").unwrap(),
        CaseId::new("case-beta").unwrap(),
        "analyst",
        RoutingMode::Tor,
        &root,
    )
    .unwrap();

    assert_ne!(sess1.profile_dir(), sess2.profile_dir());
    assert!(
        sess1
            .profile_dir()
            .starts_with(root.join("profiles").join("case-alpha"))
    );
    assert!(
        sess2
            .profile_dir()
            .starts_with(root.join("profiles").join("case-beta"))
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_8_case_isolation_enforced() {
    let root = temp_dir("test_8");
    let mut store = FileCaseStore::open(&root).unwrap();

    let case_a_id = CaseId::new("case-a").unwrap();
    let case_b_id = CaseId::new("case-b").unwrap();
    store
        .create_case(case_a_id.clone(), "Case A", "analyst")
        .unwrap();
    store
        .create_case(case_b_id.clone(), "Case B", "analyst")
        .unwrap();

    let session_a = BrowserSession::new(
        BrowserSessionId::new("sess-a").unwrap(),
        case_a_id.clone(),
        "analyst",
        RoutingMode::Direct,
        &root,
    )
    .unwrap();

    let nav_res = NavigationResult::new(
        ValidatedUrl::parse("http://example.com/target").unwrap(),
        ValidatedUrl::parse("http://example.com/target").unwrap(),
        200,
        BTreeMap::new(),
        b"Case A Classified Artifact".to_vec(),
        10,
        RoutingMode::Direct,
        false,
        false,
    );

    let captured = WebEvidenceCapturer::capture_navigation_artifact(
        &mut store,
        &session_a,
        &nav_res,
        WebArtifactKind::PageContent,
        None,
    )
    .unwrap();

    // Verify Case A has it
    let store_a = store.open_evidence_store(&case_a_id).unwrap();
    assert!(store_a.contains(&captured.address));

    // Verify Case B does NOT have it
    let store_b = store.open_evidence_store(&case_b_id).unwrap();
    assert!(!store_b.contains(&captured.address));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_9_ipc_roundtrip_lifecycle() {
    let root = temp_dir("test_9");
    let case_store = Arc::new(Mutex::new(FileCaseStore::open(&root).unwrap()));

    let case_id = CaseId::new("case-ipc").unwrap();
    {
        let mut guard = case_store.lock().unwrap();
        guard
            .create_case(case_id.clone(), "IPC Case", "analyst")
            .unwrap();
    }

    let service = BrowserService::new(root.clone());
    let engine = Arc::new(MockBrowserEngine::new());
    engine.register(
        "https://api.osint.org",
        MockResponse::Success {
            status_code: 200,
            headers: BTreeMap::new(),
            body: b"{\"ip\":\"1.2.3.4\"}".to_vec(),
            final_url: None,
        },
    );

    let mut handler =
        BrowserHandler::with_engine(service, engine).with_file_store(case_store.clone());

    // 1. Create session
    let create_req = json!({
        "session_id": "ipc-sess-1",
        "case_id": "case-ipc",
        "actor": "ipc-agent",
        "mode": "tor"
    });
    let create_res = handler
        .handle(
            &MethodName::new("browser.session.create").unwrap(),
            &create_req,
        )
        .unwrap();
    assert_eq!(create_res["id"], "ipc-sess-1");

    // 2. Navigate
    let nav_req = json!({
        "session_id": "ipc-sess-1",
        "url": "https://api.osint.org/lookup"
    });
    let nav_res = handler
        .handle(&MethodName::new("browser.navigate").unwrap(), &nav_req)
        .unwrap();
    assert_eq!(nav_res["status_code"], 200);

    // 3. Capture evidence over IPC
    let cap_req = json!({
        "session_id": "ipc-sess-1",
        "result": nav_res,
        "artifact": "page_content"
    });
    let cap_res = handler
        .handle(
            &MethodName::new("browser.evidence.capture").unwrap(),
            &cap_req,
        )
        .unwrap();
    let addr_hex = cap_res["address"].as_str().unwrap();
    assert!(!addr_hex.is_empty());

    // 4. Terminate session
    let term_req = json!({ "session_id": "ipc-sess-1" });
    let term_res = handler
        .handle(
            &MethodName::new("browser.session.terminate").unwrap(),
            &term_req,
        )
        .unwrap();
    assert_eq!(term_res["status"], "terminated");

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_10_corrupted_web_evidence_fails_closed_on_case_load() {
    let root = temp_dir("test_10");
    let mut store = FileCaseStore::open(&root).unwrap();

    let case_id = CaseId::new("case-tamper").unwrap();
    store
        .create_case(case_id.clone(), "Tamper Case", "analyst")
        .unwrap();

    let session = BrowserSession::new(
        BrowserSessionId::new("sess-tamper").unwrap(),
        case_id.clone(),
        "analyst",
        RoutingMode::Direct,
        &root,
    )
    .unwrap();

    let nav_res = NavigationResult::new(
        ValidatedUrl::parse("http://example.com/truth").unwrap(),
        ValidatedUrl::parse("http://example.com/truth").unwrap(),
        200,
        BTreeMap::new(),
        b"Original Authentic Payload".to_vec(),
        10,
        RoutingMode::Direct,
        false,
        false,
    );

    let captured = WebEvidenceCapturer::capture_navigation_artifact(
        &mut store,
        &session,
        &nav_res,
        WebArtifactKind::PageContent,
        None,
    )
    .unwrap();

    // Now tamper with the binary blob on disk
    let blob_path = root
        .join("cases")
        .join(case_id.as_str())
        .join("evidence")
        .join("blobs")
        .join(format!("{}.bin", captured.address.to_hex()));

    assert!(blob_path.exists());
    fs::write(&blob_path, b"Tampered Modified Payload").unwrap();

    // Loading the case must detect the corrupted blob and FAIL CLOSED
    let load_res = store.load_case(&case_id);
    assert!(matches!(load_res, Err(Error::IntegrityViolation(_))));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_11_sensitive_headers_scrubbed_from_provenance_and_display() {
    let mut headers = BTreeMap::new();
    headers.insert("Content-Type".into(), "application/json".into());
    headers.insert("Authorization".into(), "Bearer sk-proj-123456789".into());
    headers.insert("Cookie".into(), "session_id=super_secret_cookie".into());
    headers.insert("Proxy-Authorization".into(), "Basic dXNlcjpwYXNz".into());
    headers.insert("X-Api-Key".into(), "secret-api-key-999".into());

    let scrubbed = scrub_sensitive_headers(&headers);

    assert_eq!(scrubbed.get("Content-Type").unwrap(), "application/json");
    assert_eq!(scrubbed.get("Authorization").unwrap(), "[REDACTED]");
    assert_eq!(scrubbed.get("Cookie").unwrap(), "[REDACTED]");
    assert_eq!(scrubbed.get("Proxy-Authorization").unwrap(), "[REDACTED]");
    assert_eq!(scrubbed.get("X-Api-Key").unwrap(), "[REDACTED]");
}

#[test]
fn test_12_terminated_session_rejects_navigation() {
    let root = temp_dir("test_12");
    let mut session = BrowserSession::new(
        BrowserSessionId::new("sess-term-nav").unwrap(),
        CaseId::new("case-term").unwrap(),
        "analyst",
        RoutingMode::Direct,
        &root,
    )
    .unwrap();

    session.terminate().unwrap();
    assert!(session.status().is_terminated());

    let engine = MockBrowserEngine::new();
    let url = ValidatedUrl::parse("https://example.com").unwrap();
    let req = NavigationRequest::new(url);

    let err = engine.navigate(&session, &req).unwrap_err();
    assert!(
        err.to_string()
            .contains("cannot navigate in terminated session")
    );

    let _ = fs::remove_dir_all(&root);
}
