//! The web bridge on this Mac (same core, SQLCipher files): two web chat nodes
//! pair through `call` and talk through the mailbox functions.

#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use serde_json::{Value, json};
use vl_wasm::Web;

const T0: u64 = 1_790_000_000;

fn web(name: &str) -> Web {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("vlw-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let w = Web::open(
        dir.join("r.db").to_str().unwrap(),
        dir.join("c.db").to_str().unwrap(),
        vec![1; 32],
        vec![2; 32],
    )
    .unwrap();
    w.call("createIdentity", T0, "[]").unwrap();
    w
}

fn call(w: &Web, m: &str, now: u64, args: Value) -> Value {
    serde_json::from_str(&w.call(m, now, &args.to_string()).unwrap()).unwrap()
}

/// Mailbox stand-in: upload everything, then deliver by tag.
fn sync(w: &Web, mailbox: &mut Vec<Value>, now: u64) {
    let out = call(w, "internetOutbox", now, json!([]));
    mailbox.extend(out.as_array().unwrap().iter().cloned());
    call(w, "internetUploaded", now, json!([out]));
    for e in mailbox.clone() {
        call(w, "receiveInternet", now, json!([e]));
    }
}

#[test]
fn two_web_nodes_pair_and_talk_with_receipts() {
    let (a, b) = (web("a"), web("b"));
    let ca = call(&a, "pairingCode", T0, json!(["Asha"]));
    let cb = call(&b, "pairingCode", T0, json!(["Bala"]));
    let bala = call(&a, "completePairing", T0, json!([cb]));
    let asha = call(&b, "completePairing", T0, json!([ca]));
    assert_eq!(
        bala["displayName"], "Bala",
        "camelCase JSON, as the app expects"
    );
    assert_eq!(bala["safetyNumber"], asha["safetyNumber"]);

    let mid = call(
        &a,
        "sendMessage",
        T0,
        json!([bala["contactId"], "hello from the web", null, false]),
    );
    let mut mailbox = Vec::new();
    sync(&a, &mut mailbox, T0 + 1);
    sync(&b, &mut mailbox, T0 + 2);
    sync(&b, &mut mailbox, T0 + 3);
    sync(&a, &mut mailbox, T0 + 4);

    let got = call(&b, "conversation", T0 + 5, json!([asha["contactId"], 10]));
    assert_eq!(got[0]["body"], "hello from the web");
    let mine = call(&a, "message", T0 + 5, json!([mid]));
    assert_eq!(mine["state"], "delivered");
}

#[test]
fn errors_use_the_app_codes() {
    let a = web("e");
    let long = "x".repeat(5000);
    let err = a
        .call(
            "sendMessage",
            T0,
            &json!([vec![0u8; 16], long, null, false]).to_string(),
        )
        .unwrap_err();
    assert_eq!(err, "TextTooLong");
    let err = a
        .call(
            "sendMessage",
            T0,
            &json!([vec![0u8; 16], "hi", null, false]).to_string(),
        )
        .unwrap_err();
    assert_eq!(err, "UnknownContact");
    assert!(
        a.call("nope", T0, "[]")
            .unwrap_err()
            .starts_with("InvalidInput")
    );
    assert!(
        a.call("contacts", T0, "not json")
            .unwrap_err()
            .starts_with("InvalidInput")
    );
    let qr = call(&a, "qrCode", T0, json!(["VL1:ABC"]));
    assert!(qr["size"].as_u64().unwrap() > 0);
}
