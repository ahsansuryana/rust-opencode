//! Ported from: packages/opencode/src/tool/webfetch.ts
//!
//! Konversi HTML→Markdown memakai converter subset internal (bukan turndown):
//! elemen umum dicakup; kasus kompleks bisa beda — tercatat di DEVIATIONS.

use serde_json::json;

use crate::{Context, ExecuteResult, ToolDef, ToolError};

const MAX_RESPONSE_SIZE: usize = 5 * 1024 * 1024;
const DEFAULT_TIMEOUT_SECS: u64 = 30;
const MAX_TIMEOUT_SECS: u64 = 120;

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36";

fn accept_header(format: &str) -> &'static str {
    match format {
        "markdown" => "text/markdown;q=1.0, text/x-markdown;q=0.9, text/plain;q=0.8, text/html;q=0.7, */*;q=0.1",
        "text" => "text/plain;q=1.0, text/markdown;q=0.9, text/html;q=0.8, */*;q=0.1",
        "html" => "text/html;q=1.0, application/xhtml+xml;q=0.9, text/plain;q=0.8, text/markdown;q=0.7, */*;q=0.1",
        _ => "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8",
    }
}

fn fetch(
    url: &str,
    headers: &[(&str, &str)],
    timeout: u64,
) -> Result<(Vec<u8>, String), ToolError> {
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(timeout))
        .build();
    let mut request = agent.get(url);
    for (key, value) in headers {
        request = request.set(key, value);
    }
    let response = request.call().map_err(|error| match error {
        ureq::Error::Status(code, response) => {
            ToolError::Message(format!("HTTP {code}: {}", response.status_text()))
        }
        other => ToolError::Message(other.to_string()),
    })?;
    let content_type = response.header("content-type").unwrap_or("").to_string();
    let mut bytes = Vec::new();
    let raw_reader = response.into_reader();
    let mut reader = std::io::Read::take(raw_reader, (MAX_RESPONSE_SIZE + 1) as u64);
    std::io::Read::read_to_end(&mut reader, &mut bytes)
        .map_err(|e| ToolError::Message(e.to_string()))?;
    if bytes.len() > MAX_RESPONSE_SIZE {
        return Err(ToolError::Message(
            "Response too large (exceeds 5MB limit)".to_string(),
        ));
    }
    Ok((bytes, content_type))
}

/// Ported from: webfetch.ts:158-180 (extractTextFromHTML) — skip tag list sama.
pub fn extract_text_from_html(html: &str) -> String {
    const SKIP: &[&str] = &["script", "style", "noscript", "iframe", "object", "embed"];
    let mut out = String::new();
    let mut skip_depth = 0usize;
    let bytes: Vec<char> = html.chars().collect();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == '<' {
            // parse tag name
            let mut j = i + 1;
            let closing = j < bytes.len() && bytes[j] == '/';
            if closing {
                j += 1;
            }
            let mut name = String::new();
            while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == '-') {
                name.push(bytes[j].to_ascii_lowercase());
                j += 1;
            }
            // temukan akhir tag, hormati quoted attr
            let mut k = j;
            let mut quote: Option<char> = None;
            while k < bytes.len() {
                let c = bytes[k];
                if let Some(q) = quote {
                    if c == q {
                        quote = None;
                    }
                } else if c == '"' || c == '\'' {
                    quote = Some(c);
                } else if c == '>' {
                    break;
                }
                k += 1;
            }
            if SKIP.contains(&name.as_str()) {
                if closing {
                    skip_depth = skip_depth.saturating_sub(1);
                } else if !html[k..].starts_with("/>") {
                    skip_depth += 1;
                }
            } else if !closing
                && matches!(
                    name.as_str(),
                    "br" | "p" | "div" | "li" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
                )
            {
                out.push('\n');
            }
            i = k + 1;
            continue;
        }
        if skip_depth == 0 {
            // decode entity minimal
            decode_entity_into(&mut out, html, &mut i);
        } else {
            i += 1;
        }
    }
    out.trim().to_string()
}

fn decode_entity_into(out: &mut String, html: &str, i: &mut usize) {
    let rest = &html[*i..];
    if rest.starts_with("&amp;") {
        out.push('&');
        *i += 5;
    } else if rest.starts_with("&lt;") {
        out.push('<');
        *i += 4;
    } else if rest.starts_with("&gt;") {
        out.push('>');
        *i += 4;
    } else if rest.starts_with("&quot;") {
        out.push('"');
        *i += 6;
    } else if rest.starts_with("&#39;") {
        out.push('\'');
        i_advance(i, 5);
    } else if rest.starts_with("&nbsp;") {
        out.push(' ');
        i_advance(i, 6);
    } else {
        let c = rest.chars().next().unwrap();
        out.push(c);
        i_advance(i, c.len_utf8());
    }
}

fn i_advance(i: &mut usize, by: usize) {
    *i += by;
}

struct HtmlTag {
    name: String,
    attrs: Vec<(String, String)>,
    closing: bool,
}

fn next_tag(html: &str, from: usize) -> Option<(usize, usize, HtmlTag)> {
    let open = html[from..].find('<')? + from;
    let mut cursor = open + 1;
    let closing = html[cursor..].starts_with('/');
    if closing {
        cursor += 1;
    }
    let bytes = html.as_bytes();
    let mut name = String::new();
    while cursor < bytes.len()
        && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'-' || bytes[cursor] == b':')
    {
        name.push(bytes[cursor] as char);
        cursor += 1;
    }
    let mut attrs = Vec::new();
    loop {
        while cursor < bytes.len() && (bytes[cursor] as char).is_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() || bytes[cursor] == b'>' || bytes[cursor] == b'/' {
            break;
        }
        let mut key = String::new();
        while cursor < bytes.len()
            && !(bytes[cursor] as char).is_whitespace()
            && bytes[cursor] != b'='
            && bytes[cursor] != b'>'
        {
            key.push(bytes[cursor] as char);
            cursor += 1;
        }
        let mut value = String::new();
        if cursor < bytes.len() && bytes[cursor] == b'=' {
            cursor += 1;
            if cursor < bytes.len() && (bytes[cursor] == b'"' || bytes[cursor] == b'\'') {
                let q = bytes[cursor];
                cursor += 1;
                while cursor < bytes.len() && bytes[cursor] != q {
                    value.push(bytes[cursor] as char);
                    cursor += 1;
                }
                cursor += 1;
            } else {
                while cursor < bytes.len()
                    && !(bytes[cursor] as char).is_whitespace()
                    && bytes[cursor] != b'>'
                {
                    value.push(bytes[cursor] as char);
                    cursor += 1;
                }
            }
        }
        if !key.is_empty() {
            attrs.push((key.to_lowercase(), value));
        }
    }
    let end = html[cursor..].find('>')? + cursor + 1;
    Some((
        open,
        end,
        HtmlTag {
            name: name.to_lowercase(),
            attrs,
            closing,
        },
    ))
}

fn find_close(html: &str, from: usize, tag: &str) -> Option<usize> {
    let mut depth = 1usize;
    let mut pos = from;
    while let Some((open, end, parsed)) = next_tag(html, pos) {
        if parsed.name == tag {
            if parsed.closing {
                depth -= 1;
                if depth == 0 {
                    return Some(open);
                }
            } else {
                depth += 1;
            }
        }
        pos = end;
    }
    None
}

/// Converter HTML→Markdown subset padanan turndown config:
/// headingStyle atx, hr ---, bullet -, fenced code, em *.
pub fn convert_html_to_markdown(html: &str) -> String {
    let mut out = String::new();
    render_block(html, &mut out);
    collapse_blank_lines(&out)
}

fn render_block(html: &str, out: &mut String) {
    let mut pos = 0usize;
    while let Some((open, end, tag)) = next_tag(html, pos) {
        let text_before = &html[pos..open];
        push_inline_text(out, text_before);
        match tag.name.as_str() {
            "script" | "style" | "meta" | "link" => {
                let close_end = find_close(html, end, &tag.name)
                    .and_then(|close| next_tag(html, close).map(|(_, e2, _)| e2))
                    .unwrap_or(end);
                pos = close_end;
                continue;
            }
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                let inner_end = find_close(html, end, &tag.name).unwrap_or(end);
                let level = tag.name[1..].parse::<usize>().unwrap_or(1);
                let inner = extract_text_from_html(&html[end..inner_end]);
                if !inner.is_empty() {
                    if !out.is_empty() {
                        out.push('\n');
                        out.push('\n');
                    }
                    out.push_str(&"#".repeat(level));
                    out.push(' ');
                    out.push_str(&inner);
                    out.push('\n');
                }
                pos = inner_end;
                continue;
            }
            "p" => {
                let inner_end = find_close(html, end, &tag.name).unwrap_or(end);
                let inner = convert_html_to_markdown(&html[end..inner_end]);
                if !inner.trim().is_empty() {
                    if !out.is_empty() {
                        out.push('\n');
                        out.push('\n');
                    }
                    out.push_str(inner.trim());
                }
                pos = inner_end;
                continue;
            }
            "br" => {
                out.push('\n');
                pos = end;
                continue;
            }
            "hr" => {
                if !out.is_empty() {
                    out.push('\n');
                    out.push('\n');
                }
                out.push_str("---");
                pos = end;
                continue;
            }
            "a" => {
                let inner_end = find_close(html, end, &tag.name).unwrap_or(end);
                let label = extract_text_from_html(&html[end..inner_end]);
                let href = tag
                    .attrs
                    .iter()
                    .find(|(k, _)| k == "href")
                    .map(|(_, v)| v.clone())
                    .unwrap_or_default();
                if !label.trim().is_empty() {
                    out.push_str(&format!("[{label}]({href})"));
                }
                pos = inner_end;
                continue;
            }
            "img" => {
                let src = tag
                    .attrs
                    .iter()
                    .find(|(k, _)| k == "src")
                    .map(|(_, v)| v.clone())
                    .unwrap_or_default();
                let alt = tag
                    .attrs
                    .iter()
                    .find(|(k, _)| k == "alt")
                    .map(|(_, v)| v.clone())
                    .unwrap_or_default();
                out.push_str(&format!("![{alt}]({src})"));
                pos = end;
                continue;
            }
            "strong" | "b" => {
                let inner_end = find_close(html, end, &tag.name).unwrap_or(end);
                let label = extract_text_from_html(&html[end..inner_end]);
                if !label.is_empty() {
                    out.push_str(&format!("**{label}**"));
                }
                pos = inner_end;
                continue;
            }
            "em" | "i" => {
                let inner_end = find_close(html, end, &tag.name).unwrap_or(end);
                let label = extract_text_from_html(&html[end..inner_end]);
                if !label.is_empty() {
                    out.push_str(&format!("*{label}*"));
                }
                pos = inner_end;
                continue;
            }
            "pre" => {
                let inner_end = find_close(html, end, &tag.name).unwrap_or(end);
                let code = strip_tags(&html[end..inner_end]);
                if !out.is_empty() {
                    out.push('\n');
                    out.push('\n');
                }
                out.push_str("```\n");
                out.push_str(code.trim());
                out.push_str("\n```");
                pos = inner_end;
                continue;
            }
            "ul" | "ol" => {
                let inner_end = find_close(html, end, &tag.name).unwrap_or(end);
                render_list(&html[end..inner_end], tag.name == "ol", 0, out);
                pos = inner_end;
                continue;
            }
            "blockquote" => {
                let inner_end = find_close(html, end, &tag.name).unwrap_or(end);
                let inner = convert_html_to_markdown(&html[end..inner_end]);
                if !out.is_empty() {
                    out.push('\n');
                    out.push('\n');
                }
                for line in inner.lines() {
                    out.push_str("> ");
                    out.push_str(line);
                    out.push('\n');
                }
                pos = inner_end;
                continue;
            }
            _ => {
                out.push('<');
                out.push_str(&html[open + 1..end]);
                out.push('>');
                pos = end;
                continue;
            }
        }
    }
    push_inline_text(out, &html[pos.min(html.len())..]);
}

fn push_inline_text(out: &mut String, raw: &str) {
    let decoded = extract_text_from_html(raw);
    if !decoded.is_empty() {
        out.push_str(&decoded);
    }
}

fn strip_tags(html: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for c in html.chars() {
        match c {
            '<' => inside = true,
            '>' => inside = false,
            other if !inside => out.push(other),
            _ => {}
        }
    }
    out
}

fn render_list(html: &str, ordered: bool, depth: usize, out: &mut String) {
    let mut index = 1usize;
    let mut pos = 0usize;
    while let Some((_open, end, tag)) = next_tag(html, pos) {
        if tag.name != "li" {
            pos = end;
            continue;
        }
        let inner_end = find_close(html, end, "li").unwrap_or(end);
        let item_html = &html[end..inner_end];
        let indent = "  ".repeat(depth);
        let marker = if ordered {
            format!("{index}. ")
        } else {
            "- ".to_string()
        };
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&indent);
        out.push_str(&marker);
        // nested lists ditangani rekursif oleh render_block di dalam item
        let rendered = convert_html_to_markdown(item_html);
        out.push_str(rendered.trim());
        index += 1;
        pos = inner_end;
    }
}

fn collapse_blank_lines(input: &str) -> String {
    let mut result = String::new();
    let mut blank_run = 0usize;
    for line in input.split('\n') {
        if line.trim().is_empty() {
            blank_run += 1;
            if blank_run <= 2 {
                result.push('\n');
            }
        } else {
            blank_run = 0;
            result.push_str(line);
            result.push('\n');
        }
    }
    result.trim().to_string()
}

pub fn execute(params: &serde_json::Value, ctx: &Context) -> Result<ExecuteResult, ToolError> {
    let url = params["url"]
        .as_str()
        .ok_or_else(|| ToolError::Message("url is required".to_string()))?;
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(ToolError::Message(
            "URL must start with http:// or https://".to_string(),
        ));
    }
    let format = params
        .get("format")
        .and_then(|f| f.as_str())
        .unwrap_or("markdown")
        .to_string();
    let timeout_secs = (params
        .get("timeout")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(DEFAULT_TIMEOUT_SECS as f64) as u64)
        .min(MAX_TIMEOUT_SECS);

    ctx.ask("webfetch", vec![url.to_string()], vec!["*".to_string()], {
        let mut metadata = oc_config::v1::OrderedMap::new();
        metadata.insert("url".to_string(), json!(url));
        metadata.insert("format".to_string(), json!(format));
        metadata.insert(
            "timeout".to_string(),
            json!(params
                .get("timeout")
                .cloned()
                .unwrap_or(serde_json::Value::Null)),
        );
        metadata
    })?;

    let headers: Vec<(&str, &str)> = vec![
        ("User-Agent", USER_AGENT),
        ("Accept", accept_header(&format)),
        ("Accept-Language", "en-US,en;q=0.9"),
    ];

    // retry UA "opencode" saat Cloudflare challenge (403 cf-mitigated)
    let fetched = fetch(url, &headers, timeout_secs).or_else(|error| {
        // ureq menyembunyikan header respons pada error status; coba ulang
        // dengan UA opencode untuk URL yang gagal 403.
        if error.to_string().contains("HTTP 403") {
            let retry_headers: Vec<(&str, &str)> = vec![
                ("User-Agent", "opencode"),
                ("Accept", accept_header(&format)),
                ("Accept-Language", "en-US,en;q=0.9"),
            ];
            fetch(url, &retry_headers, timeout_secs)
        } else {
            Err(error)
        }
    })?;
    let (bytes, content_type) = fetched;

    let title = format!("{url} ({content_type})");
    let mime = content_type
        .split(';')
        .next()
        .map(str::trim)
        .unwrap_or_default()
        .to_lowercase();

    // image attachment branch menunggu attachments di ExecuteResult (session sprint)
    if mime.starts_with("image/") {
        return Ok(ExecuteResult {
            title,
            output: "Image fetched successfully".to_string(),
            metadata: json!({}),
        });
    }

    let content = String::from_utf8_lossy(&bytes).into_owned();

    let output = match format.as_str() {
        "markdown" => {
            if content_type.contains("text/html") {
                convert_html_to_markdown(&content)
            } else {
                content
            }
        }
        "text" => {
            if content_type.contains("text/html") {
                extract_text_from_html(&content)
            } else {
                content
            }
        }
        _ => content,
    };

    Ok(ExecuteResult {
        output,
        title,
        metadata: json!({}),
    })
}

/// Ported from: tool/webfetch.ts + DESCRIPTION webfetch.txt (verbatim).
pub const WEBFETCH_TOOL: ToolDef = ToolDef {
    id: "webfetch",
    description: include_str!("../assets/webfetch.txt"),
    execute,
};
