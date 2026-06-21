use scraper::{Html, Selector};
use serde::Deserialize;
use serde_json::Value;

pub enum PageKind {
    Attend,
    Confirm,
    Done,
    Unknown,
}

pub struct CodeRejection {
    errors: Vec<String>,
}

impl CodeRejection {
    pub fn from_page(html: &str) -> Self {
        let errors = extract_embedded_json(html)
            .and_then(|json| json.get("errors")?.as_array().cloned())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|value| value.as_str().map(ToOwned::to_owned))
            .collect();
        Self { errors }
    }
}

pub fn extract_embedded_json(html: &str) -> Option<Value> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("script").expect("valid script selector");

    document.select(&selector).find_map(|script| {
        let source = script.text().collect::<String>();
        let marker = "var json =";
        let start = source.find(marker)? + marker.len();
        let mut deserializer = serde_json::Deserializer::from_str(source[start..].trim_start());
        Value::deserialize(&mut deserializer).ok()
    })
}
