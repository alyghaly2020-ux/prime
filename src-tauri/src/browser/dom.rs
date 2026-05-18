use scraper::{Html, Selector};

#[derive(Debug)]
pub struct DomParser;

impl Default for DomParser {
    fn default() -> Self {
        Self::new()
    }
}

impl DomParser {
    pub fn new() -> Self {
        Self
    }

    pub fn parse_html(&self, html: &str) -> Html {
        Html::parse_document(html)
    }

    pub fn extract_text(&self, html: &str) -> String {
        let doc = Html::parse_fragment(html);
        let mut text = String::new();

        for node in doc.root_element().descendants() {
            if let Some(t) = node.value().as_text() {
                let trimmed = t.text.trim();
                if !trimmed.is_empty() {
                    if !text.is_empty() {
                        text.push(' ');
                    }
                    text.push_str(trimmed);
                }
            }
        }

        text
    }

    pub fn query_selector(&self, html: &str, css_selector: &str) -> Vec<String> {
        let doc = Html::parse_document(html);
        let selector = match Selector::parse(css_selector) {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        doc.select(&selector).map(|el| el.inner_html()).collect()
    }

    pub fn extract_links(&self, html: &str) -> Vec<(String, String)> {
        let doc = Html::parse_document(html);
        let selector = Selector::parse("a[href]")
            .expect("invalid CSS selector 'a[href]' - hardcoded selector should always be valid");

        doc.select(&selector)
            .filter_map(|el| {
                let href = el.value().attr("href")?;
                let text = el.text().collect::<String>();
                Some((href.to_string(), text))
            })
            .collect()
    }

    pub fn extract_images(&self, html: &str) -> Vec<String> {
        let doc = Html::parse_document(html);
        let selector = Selector::parse("img[src]")
            .expect("invalid CSS selector 'img[src]' - hardcoded selector should always be valid");

        doc.select(&selector)
            .filter_map(|el| el.value().attr("src").map(|s| s.to_string()))
            .collect()
    }
}
