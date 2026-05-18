use scraper::Html;

#[derive(Debug, Clone, serde::Serialize)]
pub struct A11yNode {
    pub role: String,
    pub name: String,
    pub description: Option<String>,
    pub focused: bool,
    pub enabled: bool,
    pub children: Vec<A11yNode>,
}

#[derive(Debug)]
pub struct AccessibilityTree;

impl Default for AccessibilityTree {
    fn default() -> Self {
        Self::new()
    }
}

impl AccessibilityTree {
    pub fn new() -> Self {
        Self
    }

    pub fn build(&self, html: &str) -> A11yNode {
        let doc = Html::parse_document(html);

        A11yNode {
            role: "document".to_string(),
            name: "".to_string(),
            description: None,
            focused: false,
            enabled: true,
            children: self.build_children(&doc.root_element()),
        }
    }

    fn build_children(&self, parent: &scraper::ElementRef) -> Vec<A11yNode> {
        let mut children = Vec::new();

        for child in parent.children() {
            if let Some(el) = scraper::ElementRef::wrap(child) {
                let tag = el.value().name();
                let role = self.tag_to_role(tag);
                let name = el
                    .value()
                    .attr("aria-label")
                    .or_else(|| el.value().attr("title"))
                    .or_else(|| el.value().attr("alt"))
                    .unwrap_or("")
                    .to_string();

                let description = el.value().attr("aria-description").map(|s| s.to_string());

                let focused = el.value().attr("aria-focused") == Some("true")
                    || el.value().attr("tabindex").is_some();
                let enabled = el.value().attr("aria-disabled") != Some("true");

                let node = A11yNode {
                    role,
                    name,
                    description,
                    focused,
                    enabled,
                    children: self.build_children(&el),
                };

                children.push(node);
            }
        }

        children
    }

    fn tag_to_role(&self, tag: &str) -> String {
        match tag {
            "a" => "link",
            "button" => "button",
            "input" => "textbox",
            "select" => "combobox",
            "textarea" => "textbox",
            "img" => "img",
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => "heading",
            "nav" => "navigation",
            "main" => "main",
            "header" => "banner",
            "footer" => "contentinfo",
            "form" => "form",
            "table" => "table",
            "ul" | "ol" => "list",
            "li" => "listitem",
            _ => "generic",
        }
        .to_string()
    }
}
