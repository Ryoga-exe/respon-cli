use reqwest::StatusCode;
use scraper::{ElementRef, Html, Selector};
use serde::Deserialize;
use serde_json::Value;
use url::Url;

use crate::error::{Error, Result};

pub struct Page {
    pub status: StatusCode,
    pub url: Url,
    pub body: String,
}

#[derive(PartialEq)]
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

    pub fn exists(&self) -> Option<bool> {
        if self.errors.iter().any(|error| {
            matches!(
                error.as_str(),
                "CannotDecodeCode" | "NoSuchAttendCord" | "InvalidCallNumber"
            )
        }) {
            return Some(false);
        }

        (!self.errors.is_empty() && self.errors.iter().all(|error| is_known_rejection(error)))
            .then_some(true)
    }

    pub fn status(&self) -> &'static str {
        if self.errors.iter().any(|error| error == "NoSuchAttendCord") {
            "not-found"
        } else if self
            .errors
            .iter()
            .any(|error| matches!(error.as_str(), "CannotDecodeCode" | "InvalidCallNumber"))
        {
            "invalid"
        } else if self
            .errors
            .iter()
            .any(|error| error == "InactiveCallNumber")
        {
            "inactive"
        } else if self.errors.iter().any(|error| error == "AlreadyClosed") {
            "closed"
        } else if self.errors.iter().any(|error| error == "NotYetAccepted") {
            "not-yet-open"
        } else {
            "unavailable"
        }
    }

    pub fn is_recognized(&self) -> bool {
        !self.errors.is_empty() && self.errors.iter().all(|error| is_known_rejection(error))
    }

    pub fn reason(&self) -> String {
        if self.errors.is_empty() {
            return "server returned the attendance-code page".to_owned();
        }

        self.errors
            .iter()
            .map(|error| map_rejection(error))
            .collect::<Vec<_>>()
            .join(", ")
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

pub fn card_id_in_text(value: &str) -> Option<String> {
    [
        "/auth/tsukuba/",
        "/attend-confirm/tsukuba/",
        "/complete/tsukuba/",
        "/result/tsukuba/",
    ]
    .into_iter()
    .find_map(|marker| {
        let rest = value.split_once(marker)?.1;
        let id: String = rest.chars().take_while(char::is_ascii_digit).collect();
        (!id.is_empty()).then_some(id)
    })
}

pub fn page_kind(html: &str, url: &Url) -> PageKind {
    page_kind_from_path(url.path())
        .or_else(|| {
            extract_embedded_json(html)
                .and_then(|json| json.get("url")?.as_str().map(ToOwned::to_owned))
                .and_then(|path| page_kind_from_path(&path))
        })
        .unwrap_or(PageKind::Unknown)
}

fn page_kind_from_path(path: &str) -> Option<PageKind> {
    if path.starts_with("/complete/tsukuba/") || path.starts_with("/result/tsukuba/") {
        Some(PageKind::Done)
    } else if path.starts_with("/attend-confirm/tsukuba/") {
        Some(PageKind::Confirm)
    } else if path == "/attend/tsukuba" {
        Some(PageKind::Attend)
    } else {
        None
    }
}
pub fn is_done(html: &str, url: &Url) -> bool {
    page_kind(html, url) == PageKind::Done
        || html.contains("提出済み")
        || html.to_ascii_lowercase().contains("already submitted")
}

fn is_known_rejection(error: &str) -> bool {
    matches!(
        error,
        "AnonymousNotPermitted"
            | "CannotDecodeCode"
            | "NoSuchAttendCord"
            | "InvalidCallNumber"
            | "InactiveCallNumber"
            | "AuthenticationFailed"
            | "BlankUsername"
            | "BadUsername"
            | "NotParticipant"
            | "AlreadyClosed"
            | "Interrupted"
            | "ProhibitedChatRoom"
            | "NoSubmitRoles"
            | "NotYetAccepted"
    )
}

fn map_rejection(error: &str) -> &str {
    match error {
        "AnonymousNotPermitted" => "anonymous submission is not permitted",
        "CannotDecodeCode" => "server could not decode the attendance code",
        "NoSuchAttendCord" => "attendance code does not exist",
        "InvalidCallNumber" => "attendance code is invalid",
        "InactiveCallNumber" => "attendance code is inactive",
        "AuthenticationFailed" => "authentication failed",
        "BlankUsername" => "user ID is blank",
        "BadUsername" => "user ID is invalid",
        "NotParticipant" => "user is not a participant",
        "AlreadyClosed" => "attendance acceptance is already closed",
        "Interrupted" => "attendance submission was interrupted",
        "ProhibitedChatRoom" => "chat room submission is prohibited",
        "NoSubmitRoles" => "no role can submit attendance",
        "NotYetAccepted" => "attendance acceptance has not started",
        other => other,
    }
}
