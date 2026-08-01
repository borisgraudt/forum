//! Safe Markdown → HTML for post bodies.

use pulldown_cmark::{html, Options, Parser};

/// Render Markdown to sanitized HTML (no scripts, limited tags).
pub fn render_markdown(src: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(src, options);
    let mut unsafe_html = String::new();
    html::push_html(&mut unsafe_html, parser);

    ammonia::Builder::default()
        .link_rel(Some("noopener noreferrer nofollow"))
        .clean(&unsafe_html)
        .to_string()
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
}
