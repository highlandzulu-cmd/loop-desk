//! Live smoke test of the engine bridge against the real model. Ignored by default: it needs `SOKET_API_KEY`. Run it with
//! `SOKET_API_KEY=... cargo test live_prompt_roundtrip -- --ignored --nocapture`.

use std::time::Duration;

use crate::engine::EngineHandle;
use crate::protocol::{UiCommand, UiEvent};
use crate::state::ChatRow;

#[tokio::test(flavor = "multi_thread")]
#[ignore = "talks to the real model"]
#[allow(unused_assignments)]
async fn live_prompt_roundtrip() {
    let dir = std::env::temp_dir().join("loop-desk-live-test");
    let _ = std::fs::create_dir_all(&dir);
    let (engine, mut events) = EngineHandle::spawn(dir, egui::Context::default());
    let deadline = tokio::time::Instant::now() + Duration::from_secs(180);
    let (mut started, mut ended, mut seen) = (false, None, String::new());
    let mut reply = String::new(); // only what the ASSISTANT said (not the echo of the prompt)
    loop {
        let event = tokio::time::timeout_at(deadline, events.recv())
            .await
            .expect("timed out waiting for the engine")
            .expect("engine channel closed");
        match &event {
            UiEvent::Started { .. } => {
                started = true;
                engine.send(UiCommand::Prompt("Reply with exactly the single word: pong".into()));
            }
            UiEvent::Failed(error) => panic!("engine failed: {error}"),
            UiEvent::AgentText { delta, .. } => reply.push_str(delta),
            UiEvent::Row(ChatRow::Agent { text, .. }) => reply = text.clone(),
            UiEvent::TurnEnded { reason, error } => {
                ended = Some((reason.clone(), error.clone()));
                break;
            }
            _ => {}
        }
        seen.push_str(&format!("{event:?}\n"));
    }
    assert!(started, "the engine never reported Started");
    let (reason, error) = ended.expect("no TurnEnded");
    assert!(error.is_none(), "the turn failed: {error:?} ({reason})");
    assert!(reply.to_lowercase().contains("pong"), "the assistant's reply was {reply:?}; events:\n{seen}");
    println!("assistant replied: {reply:?}");
}

/// A fresh config dir with no key: the engine asks for a login and a bogus key is rejected without being saved.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs network to reach the provider's key check"]
async fn login_rejects_a_bad_key() {
    let agent = std::env::temp_dir().join("loop-desk-login-test-agent");
    let _ = std::fs::remove_dir_all(&agent);
    // Single ignored test: nothing else reads the environment while it runs.
    unsafe {
        std::env::set_var("LOOP_CODING_AGENT_DIR", &agent);
        for key in ["SOKET_API_KEY", "OPENROUTER_API_KEY", "OPENAI_API_KEY"] {
            std::env::remove_var(key);
        }
    }
    let (engine, mut events) =
        EngineHandle::spawn(std::env::temp_dir(), egui::Context::default());
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    let mut asked = false;
    loop {
        let event = tokio::time::timeout_at(deadline, events.recv())
            .await
            .expect("timed out")
            .expect("engine channel closed");
        match event {
            UiEvent::Failed(error) => panic!("engine failed: {error}"),
            UiEvent::Models { needs_login, list, .. } => {
                assert!(needs_login && list.is_empty(), "a fresh dir should have no models");
                engine.send(UiCommand::Login {
                    provider: "soket".into(),
                    api_key: "sk-not-a-real-key".into(),
                });
                asked = true;
            }
            UiEvent::LoginResult { ok, message } => {
                assert!(asked, "result before the request");
                assert!(!ok, "a bogus key was accepted: {message}");
                println!("rejected as expected: {message}");
                break;
            }
            _ => {}
        }
    }
    let auth = agent.join("auth.json");
    assert!(
        !auth.exists() || !std::fs::read_to_string(auth).unwrap().contains("sk-not-a-real-key"),
        "a rejected key was saved"
    );
}
