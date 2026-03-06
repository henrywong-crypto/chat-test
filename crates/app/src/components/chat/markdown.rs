/// pulldown-cmark markdown → HTML renderer component.

use leptos::prelude::*;
use pulldown_cmark::{html, Options, Parser};

pub fn render_markdown_html(src: &str) -> String {
    let opts = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_SMART_PUNCTUATION;
    let parser = Parser::new_ext(src, opts);
    let mut out = String::new();
    html::push_html(&mut out, parser);
    out
}

#[component]
pub fn MarkdownRenderer(#[prop(into)] content: String) -> impl IntoView {
    let html = render_markdown_html(&content);
    view! {
        <div class="markdown-content" inner_html=html></div>
    }
}
