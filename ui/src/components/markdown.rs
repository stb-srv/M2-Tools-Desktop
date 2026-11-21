//! Markdown-Darstellung (Handbuch, Quest-Wiki, Update-Notizen) über
//! pulldown-cmark mit GFM-Tabellen. Es gibt kein Typography-Plugin - jedes
//! Element bekommt seine Tailwind-Klassen über einen [`MarkdownStyle`].
//! Rohes HTML im Markdown wird wie bei react-markdown nur als Text gezeigt.

use leptos::prelude::*;
use pulldown_cmark::{html, CowStr, Event, Options, Parser, Tag, TagEnd};

#[derive(Clone, Copy)]
pub struct MarkdownStyle {
    pub h1: &'static str,
    pub h2: &'static str,
    pub h3: &'static str,
    pub p: &'static str,
    pub ul: &'static str,
    pub ol: &'static str,
    pub a: &'static str,
    pub code: &'static str,
    pub pre: &'static str,
    pub blockquote: &'static str,
    pub table_wrap: &'static str,
    pub table: &'static str,
    pub th: &'static str,
    pub td: &'static str,
    pub hr: &'static str,
}

/// Stil des Handbuchs/Quest-Wikis (große Überschriften).
pub const DOC_STYLE: MarkdownStyle = MarkdownStyle {
    h1: "mb-4 text-2xl font-semibold",
    h2: "mb-3 mt-8 text-xl font-semibold",
    h3: "mb-2 mt-6 text-lg font-medium",
    p: "mb-4 text-sm leading-relaxed",
    ul: "mb-4 list-disc space-y-1 pl-6 text-sm",
    ol: "mb-4 list-decimal space-y-1 pl-6 text-sm",
    a: "text-primary underline",
    code: "rounded bg-muted px-1 py-0.5 font-mono text-xs",
    pre: "mb-4 overflow-x-auto rounded-md border border-border bg-muted/40 p-3",
    blockquote: "mb-4 border-l-2 border-border pl-3 text-sm text-muted-foreground",
    table_wrap: "mb-4 overflow-x-auto",
    table: "w-full border-collapse text-sm",
    th: "border border-border bg-muted/40 px-2 py-1 text-left font-medium",
    td: "border border-border px-2 py-1",
    hr: "my-6 border-border",
};

/// Kompakter Stil (Update-Notizen im "Was ist neu"-Dialog).
pub const COMPACT_STYLE: MarkdownStyle = MarkdownStyle {
    h1: "mb-3 text-lg font-semibold",
    h2: "mb-2 mt-4 text-base font-semibold",
    h3: "mb-2 mt-3 text-sm font-medium",
    p: "mb-3 text-sm leading-relaxed",
    ul: "mb-3 list-disc space-y-1 pl-5 text-sm",
    ol: "mb-3 list-decimal space-y-1 pl-5 text-sm",
    a: "text-primary underline",
    code: "rounded bg-muted px-1 py-0.5 font-mono text-xs",
    pre: "mb-3 overflow-x-auto rounded-md border border-border bg-muted/40 p-3",
    blockquote: "mb-3 border-l-2 border-border pl-3 text-sm text-muted-foreground",
    table_wrap: "mb-3 overflow-x-auto",
    table: "w-full border-collapse text-sm",
    th: "border border-border bg-muted/40 px-2 py-1 text-left font-medium",
    td: "border border-border px-2 py-1",
    hr: "my-4 border-border",
};

/// Entfernt einen YAML-Frontmatter-Block (`---\n...\n---`) am Anfang.
pub fn strip_frontmatter(src: &str) -> &str {
    let s = src.strip_prefix('\u{feff}').unwrap_or(src);
    if let Some(rest) = s.strip_prefix("---\n").or_else(|| s.strip_prefix("---\r\n")) {
        if let Some(end) = rest.find("\n---") {
            let after = &rest[end + 4..];
            return after.trim_start_matches(['\r', '\n']);
        }
    }
    s
}

/// Ziel ist ein relativer Pfad/Anker oder hat ein harmloses Schema
/// (http, https, mailto). Alles andere (`javascript:`, `data:`, `vbscript:`,
/// ...) wird nie als Link/Bild ausgegeben. Steuerzeichen und Leerraum vor/in
/// dem Schema (`java\tscript:`) werden vor der Prüfung entfernt, wie es
/// Browser beim Parsen auch tun.
pub fn is_safe_url(url: &str) -> bool {
    let cleaned: String = url.chars().filter(|c| !c.is_control() && !c.is_whitespace()).collect();
    match cleaned.find([':', '/', '?', '#']) {
        Some(i) if cleaned.as_bytes()[i] == b':' => {
            matches!(cleaned[..i].to_ascii_lowercase().as_str(), "http" | "https" | "mailto")
        }
        _ => true,
    }
}

/// Markdown -> HTML mit den Klassen aus `style`.
pub fn render_html(src: &str, style: &MarkdownStyle) -> String {
    let opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    // Links mit unsicherem Schema verlieren Start-/End-Tag (der Text bleibt),
    // Bilder verlieren das Ziel.
    let mut dropping_link = false;
    let parser = Parser::new_ext(strip_frontmatter(src), opts).filter_map(move |ev| match ev {
        Event::Html(h) | Event::InlineHtml(h) => Some(Event::Text(CowStr::from(h.to_string()))),
        Event::Start(Tag::Link { ref dest_url, .. }) if !is_safe_url(dest_url) => {
            dropping_link = true;
            None
        }
        Event::End(TagEnd::Link) if dropping_link => {
            dropping_link = false;
            None
        }
        Event::Start(Tag::Image { link_type, dest_url, title, id }) if !is_safe_url(&dest_url) => {
            Some(Event::Start(Tag::Image { link_type, dest_url: CowStr::from(""), title, id }))
        }
        other => Some(other),
    });
    let mut out = String::new();
    html::push_html(&mut out, parser);
    let c = |tag: &str, class: &str| format!("<{tag} class=\"{class}\"");
    out.replace("<pre><code>", &format!("<pre class=\"{}\"><code class=\"font-mono text-xs\">", style.pre))
        .replace("<pre><code class=\"language-", &format!("<pre class=\"{}\"><code class=\"font-mono text-xs language-", style.pre))
        .replace("<code>", &format!("{}>", c("code", style.code)))
        .replace("<h1", &c("h1", style.h1))
        .replace("<h2", &c("h2", style.h2))
        .replace("<h3", &c("h3", style.h3))
        .replace("<p>", &format!("{}>", c("p", style.p)))
        .replace("<ul>", &format!("{}>", c("ul", style.ul)))
        .replace("<ol>", &format!("{}>", c("ol", style.ol)))
        .replace("<ol start", &format!("{} start", c("ol", style.ol)))
        .replace("<a href", &format!("{} href", c("a", style.a)))
        .replace("<blockquote>", &format!("{}>", c("blockquote", style.blockquote)))
        .replace("<table>", &format!("<div class=\"{}\">{}>", style.table_wrap, c("table", style.table)))
        .replace("</table>", "</table></div>")
        .replace("<th>", &format!("{}>", c("th", style.th)))
        .replace("<th style", &format!("{} style", c("th", style.th)))
        .replace("<td>", &format!("{}>", c("td", style.td)))
        .replace("<td style", &format!("{} style", c("td", style.td)))
        .replace("<hr />", &format!("{} />", c("hr", style.hr)))
}

/// `<Markdown content=... style=DOC_STYLE/>` - rendert als HTML-Block.
#[component]
pub fn Markdown(#[prop(into)] content: Signal<String>, #[prop(optional)] style: Option<MarkdownStyle>) -> impl IntoView {
    let style = style.unwrap_or(DOC_STYLE);
    view! { <div inner_html=move || render_html(&content.get(), &style)></div> }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styles_and_escapes() {
        let h = render_html("# T\n\ntext `x` <b>raw</b>\n\n| a |\n|---|\n| 1 |\n", &DOC_STYLE);
        assert!(h.contains("<h1 class=\"mb-4 text-2xl font-semibold\">T</h1>"));
        assert!(h.contains("&lt;b&gt;raw&lt;/b&gt;"));
        assert!(h.contains("<div class=\"mb-4 overflow-x-auto\"><table class="));
        assert!(h.contains("<code class=\"rounded bg-muted"));
    }

    #[test]
    fn unsafe_link_schemes_are_not_rendered_as_links() {
        for url in ["javascript:alert(1)", "JaVaScRiPt:alert(1)", "data:text/html;base64,AAAA", "vbscript:x", " javascript:x", "java	script:x"] {
            let h = render_html(&format!("[klick]({url}) danach"), &DOC_STYLE);
            assert!(!h.contains("href"), "{url}: {h}");
            assert!(h.contains("klick"), "{url}: Text bleibt erhalten");
            assert!(h.contains("danach"));
        }
        let img = render_html("![x](javascript:alert(1))", &DOC_STYLE);
        assert!(!img.contains("javascript"), "{img}");
    }

    #[test]
    fn safe_links_stay() {
        for url in ["https://example.com/a", "http://x.de", "mailto:a@b.de", "#anker", "seite.md", "/pfad", "a/b:c", "?q=a:b"] {
            let h = render_html(&format!("[t]({url})"), &DOC_STYLE);
            assert!(h.contains("<a class="), "{url}: {h}");
        }
    }

    #[test]
    fn frontmatter() {
        assert_eq!(strip_frontmatter("---\ntitle: x\n---\n# Hi"), "# Hi");
        assert_eq!(strip_frontmatter("# Hi"), "# Hi");
    }
}
