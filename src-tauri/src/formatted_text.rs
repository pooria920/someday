use std::{borrow::Cow, collections::HashSet};

pub const MAX_HTML_BYTES: usize = 4 * 1024 * 1024;

pub fn clean(html: &str) -> Option<String> {
    if html.len() > MAX_HTML_BYTES {
        return None;
    }
    let tags: HashSet<_> = [
        "p",
        "br",
        "div",
        "span",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "strong",
        "b",
        "em",
        "i",
        "u",
        "s",
        "del",
        "ul",
        "ol",
        "li",
        "blockquote",
        "pre",
        "code",
        "table",
        "thead",
        "tbody",
        "tfoot",
        "tr",
        "th",
        "td",
        "hr",
    ]
    .into_iter()
    .collect();
    let result = ammonia::Builder::default()
        .tags(tags)
        .generic_attributes(["style"].into_iter().collect())
        .tag_attributes(Default::default())
        .attribute_filter(|_, attribute, value| {
            if attribute != "style" {
                return None;
            }
            let style = value
                .split(';')
                .filter_map(|part| {
                    let (name, value) = part.split_once(':')?;
                    let name = name.trim().to_ascii_lowercase();
                    let value = value.trim().to_ascii_lowercase();
                    match (name.as_str(), value.as_str()) {
                        ("font-weight", "bold" | "600" | "700" | "800" | "900") => {
                            Some("font-weight:bold")
                        }
                        ("font-style", "italic") => Some("font-style:italic"),
                        ("text-decoration" | "text-decoration-line", "underline") => {
                            Some("text-decoration:underline")
                        }
                        ("text-decoration" | "text-decoration-line", "line-through") => {
                            Some("text-decoration:line-through")
                        }
                        _ => None,
                    }
                })
                .collect::<Vec<_>>()
                .join(";");
            (!style.is_empty()).then_some(Cow::Owned(style))
        })
        .clean(html)
        .to_string();
    (!result.trim().is_empty() && result.len() <= MAX_HTML_BYTES).then_some(result)
}

pub fn clipboard_fragment(bytes: &[u8]) -> Option<String> {
    if bytes.len() > MAX_HTML_BYTES {
        return None;
    }
    let header_end = bytes.iter().take(1024).position(|byte| *byte == b'<')?;
    let header = std::str::from_utf8(&bytes[..header_end]).ok()?;
    let offset = |key: &str| {
        header
            .lines()
            .find_map(|line| line.strip_prefix(key))?
            .trim()
            .parse::<usize>()
            .ok()
    };
    let start_html = offset("StartHTML:")?;
    let end_html = offset("EndHTML:")?;
    let start = offset("StartFragment:")?;
    let end = offset("EndFragment:")?;
    if start_html > start || start >= end || end > end_html || end_html > bytes.len() {
        return None;
    }
    clean(std::str::from_utf8(bytes.get(start..end)?).ok()?)
}

pub fn clipboard_document(html: &str) -> String {
    let header = "Version:1.0\r\nStartHTML:0000000000\r\nEndHTML:0000000000\r\nStartFragment:0000000000\r\nEndFragment:0000000000\r\n";
    let prefix = "<html><body><!--StartFragment-->";
    let suffix = "<!--EndFragment--></body></html>";
    let start_html = header.len();
    let start = start_html + prefix.len();
    let end = start + html.len();
    let end_html = end + suffix.len();
    format!("Version:1.0\r\nStartHTML:{start_html:010}\r\nEndHTML:{end_html:010}\r\nStartFragment:{start:010}\r\nEndFragment:{end:010}\r\n{prefix}{html}{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clipboard_roundtrip_and_invalid_bounds() {
        let html = "<strong>سلام 📝</strong>";
        let document = clipboard_document(html);
        assert_eq!(
            clipboard_fragment(document.as_bytes()).as_deref(),
            Some(html)
        );
        assert!(clipboard_fragment(b"garbage").is_none());
        assert!(clipboard_fragment(
            document
                .replace("StartFragment:0000000137", "StartFragment:9999999999")
                .as_bytes()
        )
        .is_none());
        assert!(clean(&"x".repeat(MAX_HTML_BYTES + 1)).is_none());
        let bad_utf8 = clipboard_document("<b>é</b>");
        let boundary = bad_utf8.find('é').unwrap() + 1;
        let bad_utf8 = bad_utf8
            .lines()
            .map(|line| {
                if line.starts_with("StartFragment:") {
                    format!("StartFragment:{boundary:010}")
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\r\n");
        assert!(clipboard_fragment(bad_utf8.as_bytes()).is_none());
        assert!(clipboard_fragment(&vec![b'x'; MAX_HTML_BYTES + 1]).is_none());
        // The HTML body may cross the header scan bound within a multibyte character.
        let long = format!("<p>{}</p>", "ی".repeat(1000));
        assert_eq!(
            clipboard_fragment(clipboard_document(&long).as_bytes()).as_deref(),
            Some(long.as_str())
        );
    }
    #[test]
    fn removes_active_content_and_limits_styles() {
        let html = clean("<p onclick='bad()' style='color:red;font-weight:700;background:url(https://bad)'>Hello<img src='https://bad'><script>bad()</script><iframe src='https://bad'></iframe></p>").unwrap();
        assert!(html.contains("font-weight:bold"));
        for forbidden in [
            "onclick",
            "color",
            "background",
            "<img",
            "<script",
            "<iframe",
            "https://",
        ] {
            assert!(!html.contains(forbidden), "{html}");
        }
    }
}
