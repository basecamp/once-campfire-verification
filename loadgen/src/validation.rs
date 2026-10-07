//! Route-specific contracts apply to every measured response, including warmups.
use std::{io::Read, path::Path};

use bytes::Bytes;
use hyper::HeaderMap;
use regex::Regex;
use serde_json::Value;

pub struct Validator {
    kind: String,
    content_type: String,
    message_ids: Option<Vec<u64>>,
    message_content: Option<Vec<Vec<String>>>,
    required: Vec<String>,
    exact_body: Option<Vec<u8>>,
    ids: Regex,
    identity: Regex,
    presentation: Regex,
    divs: Regex,
    tags: Regex,
    words: Regex,
    closure: Regex,
    // Exact byte equality reuses validation of an immutable wire representation. No hashes,
    // unchecked samples, or cross-route entries; randomized pages are checked in full.
    valid_bodies: Vec<(String, String, Bytes)>,
    message_id: Option<u64>,
}

impl Validator {
    pub fn from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let value: Value = serde_json::from_slice(&std::fs::read(path)?)?;
        Self::from_value(value).map_err(Into::into)
    }

    fn from_value(value: Value) -> Result<Self, &'static str> {
        let kind = value["kind"].as_str().ok_or("validation contract needs kind")?.to_owned();
        if !["room_show", "messages_page", "sidebar", "search", "avatar", "static_css", "up", "post_message"].contains(&kind.as_str()) {
            return Err("unknown validation kind");
        }
        let content_type = value["content_type"].as_str().ok_or("validation contract needs content_type")?.to_owned();
        let message_ids = value
            .get("message_ids")
            .filter(|v| !v.is_null())
            .map(|v| {
                v.as_array()
                    .ok_or("message_ids must be an array")?
                    .iter()
                    .map(|v| v.as_u64().filter(|id| *id > 0).ok_or("invalid message ID"))
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?;
        if ["room_show", "messages_page", "search"].contains(&kind.as_str()) && message_ids.as_ref().is_none_or(Vec::is_empty) {
            return Err("message route needs expected IDs");
        }
        let message_content = value
            .get("message_content")
            .map(|v| {
                v.as_array()
                    .ok_or("message_content must be an array")?
                    .iter()
                    .map(|v| {
                        let tokens = v
                            .as_array()
                            .ok_or("message content must contain token arrays")?
                            .iter()
                            .map(|token| {
                                let token = token.as_str().ok_or("invalid message content token")?;
                                if token.is_empty() || !token.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
                                    return Err("invalid message content token");
                                }
                                Ok(token.to_owned())
                            })
                            .collect::<Result<Vec<_>, _>>()?;
                        if tokens.is_empty() {
                            return Err("empty message content tokens");
                        }
                        Ok(tokens)
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?;
        if ["room_show", "messages_page", "search"].contains(&kind.as_str()) && message_content.is_none() {
            return Err("message route needs expected content");
        }
        if message_content.as_ref().is_some_and(|content| Some(content.len()) != message_ids.as_ref().map(Vec::len)) {
            return Err("message content must match expected IDs");
        }
        let required = value["required"]
            .as_array()
            .ok_or("validation contract needs required markers")?
            .iter()
            .map(|v| v.as_str().map(str::to_owned).ok_or("invalid marker"))
            .collect::<Result<Vec<_>, _>>()?;
        let exact_body = value
            .get("exact_body")
            .filter(|v| !v.is_null())
            .map(|v| {
                v.as_array()
                    .ok_or("exact_body must be bytes")?
                    .iter()
                    .map(|v| v.as_u64().and_then(|v| u8::try_from(v).ok()).ok_or("invalid body byte"))
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?;
        Ok(Self {
            kind,
            content_type,
            message_ids,
            message_content,
            required,
            exact_body,
            ids: Regex::new(r#"<div\b[^>]*\sdata-message-id="(\d+)"[^>]*>"#).unwrap(),
            identity: Regex::new(r#"\sid="message_([^"]+)""#).unwrap(),
            presentation: Regex::new(r#"<div\b[^>]*\sid="presentation_message_([^"]+)"[^>]*>"#).unwrap(),
            divs: Regex::new(r"(?i)<(/?)div\b[^>]*>").unwrap(),
            tags: Regex::new(r"<[^>]*>").unwrap(),
            words: Regex::new(r"[A-Za-z0-9_]+").unwrap(),
            closure: Regex::new(r"</turbo-frame>\s*</div>\s*$").unwrap(),
            valid_bodies: Vec::new(),
            message_id: None,
        })
    }

    // Bound content to its presentation div, excluding headings, attributes and the next message.
    fn presentation_content<'a>(&self, segment: &'a str, id: u64, opener: &str) -> Result<&'a str, &'static str> {
        let presentation = self.presentation.captures(segment).ok_or("missing message presentation")?;
        let identity = self.identity.captures(opener).map(|c| c[1].to_owned());
        if presentation[1] != id.to_string() && identity.as_deref() != Some(&presentation[1]) {
            return Err("incorrect message presentation identity");
        }
        let content_start = presentation.get(0).unwrap().end();
        let mut depth = 1;
        for tag in self.divs.captures_iter(&segment[content_start..]) {
            if &tag[1] == "/" {
                depth -= 1;
                if depth == 0 {
                    return Ok(&segment[content_start..content_start + tag.get(0).unwrap().start()]);
                }
            } else {
                depth += 1;
            }
        }
        Err("incomplete message presentation")
    }

    pub fn message_id(&self) -> Option<u64> {
        self.message_id
    }

    pub fn check(&mut self, status: u16, headers: &HeaderMap, wire: &Bytes, posted: Option<&str>) -> Result<(), &'static str> {
        let content_type = headers.get("content-type").and_then(|v| v.to_str().ok()).ok_or("missing content type")?;
        let encoding = headers.get("content-encoding").map(|v| v.to_str().unwrap_or("invalid")).unwrap_or("identity");
        if status != 200 || wire.is_empty() {
            return Err("unsuccessful or empty response");
        }
        if content_type.split(';').next().unwrap_or("").trim() != self.content_type {
            return Err("wrong content type");
        }
        if let Some(length) = headers.get("content-length")
            && length.to_str().ok().and_then(|s| s.parse::<usize>().ok()) != Some(wire.len())
        {
            return Err("wrong content length");
        }
        if self.kind != "post_message" && self.valid_bodies.iter().any(|(e, t, b)| e == encoding && t == content_type && b == wire) {
            return Ok(());
        }
        let decoded;
        let body = match encoding {
            "identity" => wire.as_ref(),
            "gzip" => {
                let mut decoder = flate2::bufread::GzDecoder::new(wire.as_ref());
                let mut output = Vec::new();
                decoder.by_ref().take(8 * 1024 * 1024 + 1).read_to_end(&mut output).map_err(|_| "invalid gzip stream")?;
                if output.len() > 8 * 1024 * 1024 || !decoder.into_inner().is_empty() {
                    return Err("oversized or trailing gzip data");
                }
                decoded = output;
                &decoded
            }
            _ => return Err("unsupported content encoding"),
        };
        if let Some(expected) = &self.exact_body {
            if body != expected {
                return Err("changed binary or static response");
            }
        } else {
            let text = std::str::from_utf8(body).map_err(|_| "invalid UTF-8")?;
            if self.required.iter().any(|marker| !text.contains(marker)) {
                return Err("missing expected content");
            }
            if ["room_show", "sidebar", "search"].contains(&self.kind.as_str())
                && (!text.trim_start().get(..15).is_some_and(|v| v.eq_ignore_ascii_case("<!doctype html>"))
                    || !text.trim_end().ends_with("</html>"))
            {
                return Err("incomplete HTML document");
            }
            let messages = self
                .ids
                .captures_iter(text)
                .map(|c| c[1].parse::<u64>().ok().filter(|id| *id > 0).map(|id| (id, c.get(0).unwrap())).ok_or("invalid message ID"))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| "invalid message ID")?;
            let ids = messages.iter().map(|(id, _)| *id).collect::<Vec<_>>();
            if let Some(expected) = &self.message_ids
                && &ids != expected
            {
                return Err("incorrect message window");
            }
            if let Some(expected) = &self.message_content {
                for (i, tokens) in expected.iter().enumerate() {
                    let (id, opener) = messages[i];
                    let end = messages.get(i + 1).map_or(text.len(), |(_, opener)| opener.start());
                    let segment = &text[opener.end()..end];
                    if self.kind == "messages_page" && !self.closure.is_match(segment) {
                        return Err("incomplete paginated message");
                    }
                    let content = self.presentation_content(segment, id, opener.as_str())?;
                    let plain = self.tags.replace_all(content, " ");
                    let mut actual = self.words.find_iter(&plain);
                    for token in tokens {
                        if !actual.by_ref().any(|word| word.as_str() == token) {
                            return Err("missing seeded message content");
                        }
                    }
                }
            }
            if self.kind == "post_message" {
                if !text.starts_with("<turbo-stream")
                    || !text.trim_end().ends_with("</turbo-stream>")
                    || !text.contains("<template>")
                    || !text.contains("</template>")
                    || ids.len() != 1
                {
                    return Err("invalid message Turbo Stream");
                }
                self.message_id = Some(ids[0]);
                if !posted.is_some_and(|token| text.contains(token)) {
                    return Err("POST did not render the requested message");
                }
            }
        }
        if self.kind != "post_message" {
            if self.valid_bodies.len() == 8 {
                self.valid_bodies.remove(0);
            }
            self.valid_bodies.push((encoding.to_owned(), content_type.to_owned(), wire.clone()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write;

    fn validator(kind: &str) -> Validator {
        let content_type = if kind == "post_message" { "text/vnd.turbo-stream.html" } else { "text/html" };
        let mut contract = json!({"kind":kind,"content_type":content_type,"required":[]});
        if kind == "room_show" {
            contract["message_ids"] = json!([42]);
            contract["message_content"] = json!([["coffee"]]);
        }
        Validator::from_value(contract).unwrap()
    }
    fn headers(content_type: &str, gzip: bool) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert("content-type", content_type.parse().unwrap());
        if gzip {
            h.insert("content-encoding", "gzip".parse().unwrap());
        }
        h
    }
    fn message(id: u64, body: &str) -> String {
        format!(
            r#"<div id="message_client-{id}" data-message-id="{id}"><h3>Heading</h3><turbo-frame id="edit_message_client-{id}"><div id="presentation_message_client-{id}"><p>{body}</p></div></turbo-frame></div>"#
        )
    }
    fn page(body: &str) -> Bytes {
        Bytes::from(format!("<!DOCTYPE html><html>{body}</html>"))
    }
    fn messages_validator() -> Validator {
        Validator::from_value(json!({"kind":"messages_page","content_type":"text/html",
            "message_ids":[42,43],"message_content":[["001","Coffee","machine","is","fixed"],["002","meeting"]],"required":[]}))
        .unwrap()
    }
    #[test]
    fn rejects_200_error_pages_truncated_pages_and_wrong_message_windows() {
        let mut v = validator("room_show");
        let h = headers("text/html", false);
        let good = page(&message(42, "coffee"));
        assert!(v.check(200, &h, &good, None).is_ok());
        assert!(v.check(200, &h, &good, None).is_ok());
        for bad in [Bytes::from_static(b"error"), Bytes::from(message(42, "coffee")), page(&message(43, "coffee")), page("")] {
            assert!(v.check(200, &h, &bad, None).is_err());
        }
        assert!(v.check(500, &h, &good, None).is_err());
        assert!(v.check(200, &headers("text/plain", false), &good, None).is_err());
    }
    #[test]
    fn validates_gzip_crc_truncation_trailing_data_and_content_after_cache_hit() {
        let mut v = validator("room_show");
        let h = headers("text/html", true);
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        gz.write_all(&page(&message(42, "coffee"))).unwrap();
        let bytes = gz.finish().unwrap();
        assert!(v.check(200, &h, &Bytes::from(bytes.clone()), None).is_ok());
        let mut corrupt = bytes.clone();
        let n = corrupt.len();
        corrupt[n - 8] ^= 1;
        assert!(v.check(200, &h, &Bytes::from(corrupt), None).is_err());
        assert!(v.check(200, &h, &Bytes::copy_from_slice(&bytes[..bytes.len() - 1]), None).is_err());
        let mut trailing = bytes;
        trailing.push(0);
        assert!(v.check(200, &h, &Bytes::from(trailing), None).is_err());
    }
    #[test]
    fn checks_all_seeded_words_and_ordinals_in_order_in_each_presentation() {
        let mut v = messages_validator();
        let h = headers("text/html", false);
        let first = message(42, "001. Coffee <strong>machine</strong> is fixed!");
        let second = message(43, "002. meeting");
        assert!(v.check(200, &h, &Bytes::from(format!("{first}{second}")), None).is_ok());
        for body in
            ["machine", "Coffee machine is fixed", "001. Coffee is fixed", "001. fixed is machine Coffee", "001. Coffee machines is fixed"]
        {
            assert!(v.check(200, &h, &Bytes::from(format!("{}{second}", message(42, body))), None).is_err(), "{body}");
        }
        let moved = format!("{}{}", message(42, ""), message(43, "001 Coffee machine is fixed 002 meeting"));
        assert!(v.check(200, &h, &Bytes::from(moved), None).is_err());
        let wrong_heading = first
            .replace("<h3>Heading</h3>", "<h3>001 Coffee machine is fixed</h3>")
            .replace("<p>001. Coffee <strong>machine</strong> is fixed!</p>", "<p></p>");
        assert!(v.check(200, &h, &Bytes::from(format!("{wrong_heading}{second}")), None).is_err());
        let attribute_only =
            first.replace("<p>001. Coffee <strong>machine</strong> is fixed!</p>", r#"<a title="001 Coffee machine is fixed"></a>"#);
        assert!(v.check(200, &h, &Bytes::from(format!("{attribute_only}{second}")), None).is_err());
        let wrong_presentation = first.replace("presentation_message_client-42", "presentation_message_client-43");
        assert!(v.check(200, &h, &Bytes::from(format!("{wrong_presentation}{second}")), None).is_err());
    }
    #[test]
    fn rejects_truncated_final_pagination_message_after_all_expected_content() {
        let mut v = messages_validator();
        let h = headers("text/html", false);
        let full = format!("{}{}", message(42, "001 Coffee machine is fixed"), message(43, "002 meeting"));
        assert!(v.check(200, &h, &Bytes::from(full.clone()), None).is_ok());
        for suffix in ["</p></div></turbo-frame></div>", "</div></turbo-frame></div>", "</turbo-frame></div>", "</div>"] {
            assert!(v.check(200, &h, &Bytes::from(full.strip_suffix(suffix).unwrap().to_owned()), None).is_err(), "{suffix}");
        }
        let first = message(42, "001 Coffee machine is fixed").replace("</turbo-frame>", "");
        assert!(v.check(200, &h, &Bytes::from(format!("{first}{}", message(43, "002 meeting"))), None).is_err());
    }
    #[test]
    fn rejects_next_opening_attributes_as_missing_content_and_zero_or_overflow_ids() {
        let mut v = messages_validator();
        let h = headers("text/html", false);
        let body = format!(
            "{}{}",
            message(42, "001 Coffee machine is"),
            message(43, "002 meeting").replace("data-message-id", r#"title="fixed" data-message-id"#)
        );
        assert!(v.check(200, &h, &Bytes::from(body), None).is_err());
        let mut v = validator("room_show");
        for id in ["0", "18446744073709551616"] {
            let bad = page(&message(42, "coffee").replace(r#"data-message-id="42""#, &format!(r#"data-message-id="{id}""#)));
            assert!(v.check(200, &h, &bad, None).is_err());
        }
        let mut post = validator("post_message");
        let body = Bytes::from(format!("<turbo-stream><template>{}</template></turbo-stream>", message(0, "bench write unique")));
        assert!(post.check(200, &headers("text/vnd.turbo-stream.html", false), &body, Some("bench write unique")).is_err());
    }
    #[test]
    fn post_requires_actual_message_and_per_request_body() {
        let mut v = validator("post_message");
        let h = headers("text/vnd.turbo-stream.html", false);
        let body = Bytes::from(format!(
            "<turbo-stream action=\"append\"><template>{}</template></turbo-stream>",
            message(42, "bench write unique")
        ));
        assert!(v.check(200, &h, &body, Some("bench write unique")).is_ok());
        assert!(v.check(200, &h, &body, Some("bench write different")).is_err());
        assert!(
            v.check(
                200,
                &h,
                &Bytes::from_static(b"<turbo-stream><template>This room was deleted.</template></turbo-stream>"),
                Some("bench write unique")
            )
            .is_err()
        );
    }
    #[test]
    fn invalid_or_thin_validation_contract_fails_closed() {
        for contract in [
            json!({"kind":"room_show","content_type":"text/html","required":[]}),
            json!({"kind":"unknown","content_type":"text/html","required":[]}),
            json!({"kind":"room_show","content_type":"text/html","message_ids":[42],"required":[]}),
            json!({"kind":"messages_page","content_type":"text/html","message_ids":[42],"message_content":[["coffee"],["meeting"]],"required":[]}),
            json!({"kind":"room_show","content_type":"text/html","message_ids":[0],"message_content":[["coffee"]],"required":[]}),
            json!({"kind":"room_show","content_type":"text/html","message_ids":[42],"message_content":[[]],"required":[]}),
            json!({"kind":"room_show","content_type":"text/html","message_ids":[42],"message_content":[[""]],"required":[]}),
            json!({"kind":"room_show","content_type":"text/html","message_ids":[42],"message_content":["coffee"],"required":[]}),
        ] {
            assert!(Validator::from_value(contract).is_err());
        }
    }
}
