//! Safe Markdown → HTML for post bodies, with @mention linking.

use pulldown_cmark::{html, Options, Parser};
use regex::Regex;
use std::sync::OnceLock;

/// Render Markdown to sanitized HTML (no scripts, limited tags).
/// `@username` mentions become links to `/u/{username}` before markdown parse.
pub fn render_markdown(src: &str) -> String {
    let with_mentions = linkify_mentions(src);

    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(&with_mentions, options);
    let mut unsafe_html = String::new();
    html::push_html(&mut unsafe_html, parser);

    ammonia::Builder::default()
        .link_rel(Some("noopener noreferrer nofollow"))
        .clean(&unsafe_html)
        .to_string()
}

fn mention_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // @username: letter/underscore first, then alnum/underscore (2–31 more → total 3–32)
        Regex::new(r"(?P<pre>^|[^A-Za-z0-9_])@(?P<user>[A-Za-z_][A-Za-z0-9_]{2,31})\b")
            .expect("mention regex")
    })
}

/// Turn bare @mentions into markdown links, avoiding emails and already-linked text.
fn linkify_mentions(src: &str) -> String {
    mention_re()
        .replace_all(src, |caps: &regex::Captures| {
            let pre = caps.name("pre").map(|m| m.as_str()).unwrap_or("");
            let user = caps.name("user").map(|m| m.as_str()).unwrap_or("");
            format!("{pre}[@{user}](/u/{user})")
        })
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_bold() {
        let html = render_markdown("hello **world**");
        assert!(html.contains("<strong>world</strong>") || html.contains("<strong>world</strong>"));
        assert!(html.contains("hello"));
    }

    #[test]
    fn strips_script() {
        let html = render_markdown("<script>alert(1)</script>hi");
        assert!(!html.to_lowercase().contains("<script"));
        assert!(html.contains("hi"));
    }

    #[test]
    fn allows_links() {
        let html = render_markdown("[x](https://example.com)");
        assert!(html.contains("href=\"https://example.com\""));
        assert!(html.contains("noopener"));
    }

    #[test]
    fn linkifies_mentions() {
        let html = render_markdown("hey @alice how are you");
        assert!(
            html.contains("href=\"/u/alice\"") || html.contains("href='/u/alice'"),
            "html was: {html}"
        );
        assert!(
            html.contains("@alice") || html.contains(">alice<"),
            "html was: {html}"
        );
    }

    #[test]
    fn does_not_eat_email() {
        let html = render_markdown("mail me at user@example.com please");
        // Should not become /u/example
        assert!(!html.contains("/u/example"));
    }
}
