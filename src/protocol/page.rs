use reqwest::StatusCode;
use scraper::{Element, ElementRef, Html, Selector};
use serde::Deserialize;
use serde_json::Value;
use url::Url;

use crate::error::{Error, Result};

pub struct Page {
    pub status: StatusCode,
    pub url: Url,
    pub body: String,
}

pub enum PageKind {
    Attend,
    Confirm,
    Done,
    Unknown,
}

pub struct ParsedForm {
    pub method: String,
    pub action: String,
    pub fields: Vec<(String, String)>,
}

impl ParsedForm {
    pub fn contains(&self, name: &str) -> bool {
        self.fields.iter().any(|(field, _)| field == name)
    }

    pub fn set(&mut self, name: &str, value: impl Into<String>) {
        self.fields.retain(|(field, _)| field != name);
        self.fields.push((name.to_owned(), value.into()));
    }
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

pub fn extract_forms(html: &str) -> Vec<ParsedForm> {
    let document = Html::parse_document(html);
    let form_selector = Selector::parse("form").expect("valid form selector");
    let control_selector =
        Selector::parse("input[name], button[name]").expect("valid control selector");

    document
        .select(&form_selector)
        .map(|form| {
            let fields = form
                .select(&control_selector)
                .filter_map(parse_control)
                .collect();
            ParsedForm {
                method: form
                    .value()
                    .attr("method")
                    .unwrap_or("get")
                    .to_ascii_lowercase(),
                action: form.value().attr("action").unwrap_or("").to_owned(),
                fields,
            }
        })
        .collect()
}

fn parse_control(control: ElementRef<'_>) -> Option<(String, String)> {
    let name = control.value().attr("name")?.to_owned();
    let input_type = control.value().attr("type").unwrap_or("text");
    if matches!(input_type, "checkbox" | "radio") && control.value().attr("checked").is_none() {
        return None;
    }
    Some((name, control.value().attr("value").unwrap_or("").to_owned()))
}

pub fn extract_authenticity_token(html: &str) -> Result<String> {
    if let Some(token) = extract_embedded_json(html).and_then(|json| {
        json.get("authenticity_token")?
            .as_str()
            .map(ToOwned::to_owned)
    }) {
        return Ok(token);
    }

    extract_forms(html)
        .into_iter()
        .flat_map(|form| form.fields)
        .find_map(|(name, value)| (name == "authenticity_token").then_some(value))
        .ok_or_else(|| Error::Protocol("authenticity_token was not found".to_owned()))
}
