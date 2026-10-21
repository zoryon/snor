use serde_json::{json, Value};

pub const MAX_FRAME: usize = 4 * 1024 * 1024;
pub const MAX_OUTPUT: usize = 2 * 1024 * 1024;

pub fn initialize(id: &str) -> Value {
    json!({"type":"control_request","request_id":id,"request":{"subtype":"initialize","hooks":{}}})
}

/// Takes the attachments by value: base64 payloads are moved into the frame, not copied.
pub fn user_message(text: &str, attachments: Vec<Value>) -> Value {
    let mut content = Vec::with_capacity(attachments.len() + 1);
    content.push(json!({"type":"text","text":text}));
    for mut a in attachments {
        if a["kind"] == "image" {
            content.push(json!({"type":"image","source":{"type":"base64","media_type":a["mime"].take(),"data":a["data"].take()}}));
        } else if a["kind"] == "text" {
            content.push(json!({"type":"text","text":format!("\n--- File allegato: {} ---\n{}",a["name"].as_str().unwrap_or("file"),a["data"].as_str().unwrap_or(""))}));
        }
    }
    json!({"type":"user","message":{"role":"user","content":content},"parent_tool_use_id":null})
}

pub fn permission_response(id: &str, input: Value, allow: bool) -> Value {
    let decision = if allow {
        json!({"behavior":"allow","updatedInput":input})
    } else {
        json!({"behavior":"deny","message":"Azione rifiutata dall'utente in Snor."})
    };
    json!({"type":"control_response","response":{"subtype":"success","request_id":id,"response":decision}})
}

pub fn unsupported_response(id: &str) -> Value {
    json!({"type":"control_response","response":{"subtype":"error","request_id":id,"error":"Richiesta di controllo non supportata da questa versione di Snor."}})
}

pub fn control(id: &str, subtype: &str) -> Value {
    json!({"type":"control_request","request_id":id,"request":{"subtype":subtype}})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn allow_and_deny_use_nested_request_ids() {
        let input = json!({"command":"echo hello"});
        let allowed = permission_response("p1", input.clone(), true);
        assert_eq!(allowed["response"]["request_id"], "p1");
        assert_eq!(allowed["response"]["response"]["updatedInput"], input);
        assert_eq!(
            permission_response("p1", json!({}), false)["response"]["response"]["behavior"],
            "deny"
        );
    }
    #[test]
    fn multimodal_inputs_are_structured_and_not_shell_commands() {
        let msg = user_message(
            "$(secret)",
            vec![
                json!({"kind":"text","name":"a.txt","data":"hello"}),
                json!({"kind":"image","mime":"image/png","data":"AAAA"}),
            ],
        );
        assert_eq!(msg["message"]["content"][0]["text"], "$(secret)");
        assert_eq!(msg["message"]["content"][2]["source"]["type"], "base64");
    }
}
