//! Disposable native/browser framing shared by format adapters, away from audio.
//! This gate consumes no request and grants no session or filesystem authority.
//! The caller still checks its live binding and the owner's negotiated limits,
//! then routes the unchanged frame through shared command admission.
use crate::protocol::PATCH_MESSAGE_BYTES_LIMIT;
use std::num::NonZeroU64;

pub struct BrowserRequest<'a> {
    request_id: String,
    native_selection: bool,
    message: &'a str,
}
impl BrowserRequest<'_> {
    pub fn request_id(&self) -> &str {
        &self.request_id
    }
    pub fn is_native_selection(&self) -> bool {
        self.native_selection
    }
    pub fn message(&self) -> &str {
        self.message
    }
}

/// Preserve the existing silent rejection of uncorrelatable native IPC.
/// Semantic versions/methods/arguments remain the command owner's decision.
pub fn browser_request<'a>(attachment: &str, message: &'a str) -> Option<BrowserRequest<'a>> {
    if message.len() > PATCH_MESSAGE_BYTES_LIMIT {
        return None;
    }
    let frame: serde_json::Value = serde_json::from_str(message).ok()?;
    if frame["attachmentId"].as_str() != Some(attachment) {
        return None;
    }
    let request_id = frame["requestId"].as_str()?;
    if request_id.len() > 20 {
        return None;
    }
    let id = request_id.parse::<u64>().ok()?;
    if id == 0 || id.to_string() != request_id {
        return None;
    }
    Some(BrowserRequest {
        request_id: request_id.to_owned(),
        native_selection: frame["method"].as_str() == Some("chooseSamplerFiles"),
        message,
    })
}

/// Initialize a browser with the adapter's already validated live namespace.
pub fn initialization_script(
    instance: &str,
    attachment: &str,
    next_request: NonZeroU64,
    middle_c_octave: u8,
) -> String {
    format!(
        "window.__DANDRUM_NATIVE__={};",
        serde_json::json!({"protocolVersion":1,"instanceId":instance,"attachmentId":attachment,"nextRequestId":next_request.to_string(),"middleCOctave":middle_c_octave})
    )
}

/// Deliver one JSON STRING to the broker; script queueing is not consumption.
pub fn delivery_script(frame: &str) -> String {
    format!(
        "window.__DANDRUM__.receiveNativeMessage({});",
        serde_json::to_string(frame).expect("String JSON serialization")
    )
}
