#![no_main]

use jira_core::adf::{adf_to_text, inject_mentions, markdown_to_adf, plain_text_to_adf};
use libfuzzer_sys::fuzz_target;
use serde_json::Value;

fuzz_target!(|data: &[u8]| {
    if data.len() > 4096 {
        return;
    }

    let text = String::from_utf8_lossy(data);

    let adf = markdown_to_adf(&text);
    let rendered = adf_to_text(&adf);
    let plain = plain_text_to_adf(&rendered);
    let _ = adf_to_text(&plain);

    let mut with_mentions = adf.clone();
    let mention_map = vec![
        ("alice".to_string(), "acct-alice".to_string()),
        ("bob".to_string(), "acct-bob".to_string()),
    ];
    inject_mentions(&mut with_mentions, &mention_map);
    let _ = adf_to_text(&with_mentions);

    if let Ok(mut json_value) = serde_json::from_slice::<Value>(data) {
        inject_mentions(&mut json_value, &mention_map);
        let _ = adf_to_text(&json_value);
    }
});
