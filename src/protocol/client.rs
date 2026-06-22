use std::{sync::Arc, time::Duration};

use reqwest::{
    blocking::{Client, Response},
    cookie::Jar,
    header::{ACCEPT, ACCEPT_LANGUAGE, LOCATION, ORIGIN, REFERER},
    redirect::Policy,
};
use url::Url;
use zeroize::Zeroizing;

use crate::{
    diagnostics::Diagnostics,
    error::{Error, Result},
    protocol::{
        page::{
            CodeRejection, ConfirmationSpec, Page, card_id_in_text, completion,
            extract_authenticity_token, extract_confirmation, is_done,
        },
        status::{AttendanceAccess, PreparationStatus, ProbeStatus, SubmissionResponse},
    },
};

const ATMNB_URL: &str = "https://atmnb.tsukuba.ac.jp";
const ATTEND_URL: &str = "https://atmnb.tsukuba.ac.jp/attend/tsukuba";
const IDP_HOST: &str = "idp.account.tsukuba.ac.jp";
const DEFAULT_USER_AGENT: &str = concat!("respon-cli/", env!("CARGO_PKG_VERSION"));

pub struct Credentials {
    pub username: String,
    pub password: Zeroizing<String>,
}

enum ProbeRedirect {
    Available(AttendanceAccess),
    Rejected(Url),
}

pub struct ResponClient {
    follow: Client,
    no_redirect: Client,
    diagnostics: Diagnostics,
}

impl ResponClient {
    pub fn new(diagnostics: Diagnostics, user_agent: Option<&str>) -> Result<Self> {
        let jar = Arc::new(Jar::default());
        let user_agent = user_agent.unwrap_or(DEFAULT_USER_AGENT);
        let follow = build_client(jar.clone(), Policy::limited(20), user_agent)?;
        let no_redirect = build_client(jar, Policy::none(), user_agent)?;
        Ok(Self {
            follow,
            no_redirect,
            diagnostics,
        })
    }

    pub fn probe_code(&self, code: &str) -> Result<ProbeStatus> {
        let attend_url = Url::parse(ATTEND_URL)?;
        let response = self.follow.get(attend_url.clone()).send()?;
        response.error_for_status_ref()?;
        self.diagnostics.log(format!(
            "GET {ATTEND_URL} -> {} {}",
            response.status(),
            response.url()
        ));
        let token = extract_authenticity_token(&response.text()?)?;
        self.diagnostics
            .log(format!("authenticity_token length={}", token.len()));

        let fields = [
            ("authenticity_token", token.as_str()),
            ("code", code),
            ("insertdb", "GO"),
        ];
        let response = self
            .no_redirect
            .post(attend_url.clone())
            .header(ORIGIN, ATMNB_URL)
            .header(REFERER, ATTEND_URL)
            .form(&fields)
            .send()?;
        response.error_for_status_ref()?;
        let status = response.status();
        let location = response
            .headers()
            .get(LOCATION)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        self.diagnostics.log(format!(
            "POST {ATTEND_URL} -> {status} location={}",
            location.as_deref().unwrap_or("-")
        ));

        let location = location.ok_or_else(|| {
            Error::Protocol(format!(
                "attendance-code request did not redirect: HTTP {status}"
            ))
        })?;
        let location = attend_url.join(&location)?;

        match classify_probe_redirect(location)? {
            ProbeRedirect::Available(access) => Ok(ProbeStatus::Available(access)),
            ProbeRedirect::Rejected(location) => {
                let response = self.follow.get(location).send()?;
                let Page { body, url, .. } = page_from_response(response)?;
                if is_done(&body, &url) {
                    return Err(Error::Protocol(format!(
                        "attendance-code rejection unexpectedly returned a completion page: {}",
                        url
                    )));
                }
                Ok(ProbeStatus::Unavailable(CodeRejection::from_page(&body)))
            }
        }
    }

    pub fn prepare_after_authentication(
        &self,
        login_url: &Url,
        credentials: &Credentials,
    ) -> Result<PreparationStatus> {
        let page = self.authenticate(login_url.as_str(), credentials, "card")?;
        self.preparation_status(page)
    }

    pub fn prepare_confirmation(&self, page_url: &Url) -> Result<PreparationStatus> {
        let response = self.follow.get(page_url.clone()).send()?;
        self.preparation_status(page_from_response(response)?)
    }

    fn preparation_status(&self, page: Page) -> Result<PreparationStatus> {
        self.diagnostics
            .log(format!("card auth final -> {} {}", page.status, page.url));

        if is_done(&page.body, &page.url) {
            return Ok(PreparationStatus::AlreadySubmitted {
                completion: completion(&page),
                url: page.url,
            });
        }

        Ok(PreparationStatus::Confirmation(extract_confirmation(
            &page,
        )?))
    }

    pub fn submit(&self, confirmation: &ConfirmationSpec) -> Result<SubmissionResponse> {
        let response = self
            .follow
            .post(confirmation.action.clone())
            .header(ORIGIN, ATMNB_URL)
            .header(REFERER, confirmation.action.as_str())
            .form(&confirmation.fields)
            .send()?;
        let page = page_from_response(response)?;
        self.diagnostics
            .log(format!("submit final -> {} {}", page.status, page.url));

        let completed = completion(&page).ok_or_else(|| {
            Error::Protocol(format!(
                "submission did not reach a completion page: {}",
                page.url
            ))
        })?;
        Ok(SubmissionResponse {
            url: page.url,
            completion: completed,
        })
    }

    fn authenticate(
        &self,
        start_url: &str,
        credentials: &Credentials,
        label: &str,
    ) -> Result<Page> {
        todo!("implement")
    }
}

fn build_client(jar: Arc<Jar>, redirect: Policy, user_agent: &str) -> Result<Client> {
    Ok(Client::builder()
        .cookie_provider(jar)
        .redirect(redirect)
        .timeout(Duration::from_secs(30))
        .user_agent(user_agent)
        .default_headers({
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                ACCEPT,
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"
                    .parse()
                    .expect("valid Accept header"),
            );
            headers.insert(
                ACCEPT_LANGUAGE,
                "ja-JP,ja;q=0.9".parse().expect("valid language header"),
            );
            headers
        })
        .build()?)
}

fn classify_probe_redirect(location: Url) -> Result<ProbeRedirect> {
    if location.path().starts_with("/attend-confirm/tsukuba/") {
        let card_id = redirect_card_id(&location)?;
        return Ok(ProbeRedirect::Available(
            AttendanceAccess::ConfirmationAvailable {
                card_id,
                page_url: location,
            },
        ));
    }

    if location.path() == "/ct/attend/pc" {
        let card_id = redirect_card_id(&location)?;
        return Ok(ProbeRedirect::Available(
            AttendanceAccess::AuthenticationRequired {
                card_id,
                login_url: location,
            },
        ));
    }

    if location.path() == "/attend/tsukuba" {
        return Ok(ProbeRedirect::Rejected(location));
    }

    if location.path().starts_with("/complete/tsukuba/")
        || location.path().starts_with("/result/tsukuba/")
    {
        return Err(Error::Protocol(format!(
            "attendance-code request reached a user-specific page before authentication: {location}"
        )));
    }

    Err(Error::Protocol(format!(
        "unexpected attendance-code redirect: {location}"
    )))
}

fn redirect_card_id(location: &Url) -> Result<String> {
    card_id_in_text(location.as_str()).ok_or_else(|| {
        Error::Protocol(format!(
            "card ID was not found in attendance-code redirect: {location}"
        ))
    })
}

fn page_from_response(response: Response) -> Result<Page> {
    response.error_for_status_ref()?;
    let status = response.status();
    let url = response.url().clone();
    let body = response.text()?;
    Ok(Page { status, url, body })
}
