use std::{sync::Arc, time::Duration};

use reqwest::{
    Url,
    blocking::Client,
    cookie::Jar,
    header::{ACCEPT, ACCEPT_LANGUAGE, LOCATION, ORIGIN, REFERER},
    redirect::Policy,
};

use crate::{
    diagnostics::Diagnostics,
    error::{Error, Result},
};

const ATMNB_URL: &str = "https://atmnb.tsukuba.ac.jp";
const MANABA_HOME_URL: &str = "https://manaba.tsukuba.ac.jp/ct/home";
const ATTEND_URL: &str = "https://atmnb.tsukuba.ac.jp/attend/tsukuba";
const IDP_HOST: &str = "idp.account.tsukuba.ac.jp";
const DEFAULT_USER_AGENT: &str = concat!("respon-cli/", env!("CARGO_PKG_VERSION"));

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

    pub fn check(&self, code: &str) -> Result<bool> {
        self.submit_code(code)
    }

    fn submit_code(&self, code: &str) -> Result<bool> {
        let attend_url = Url::parse(ATTEND_URL)?;
        let response = self.follow.get(attend_url.clone()).send()?;
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

        let absolute = location
            .as_deref()
            .map(|value| attend_url.join(value))
            .transpose()?;
        if let Some(location) = absolute {
            if location.path().starts_with("/complete/tsukuba/")
                || location.path().starts_with("/result/tsukuba/")
            {
                let response = self.follow.get(location);
                // AlreadySubmitted
                return Ok(true);
            }

            if location.path().starts_with("/attend-confirm/tsukuba/") {
                todo!("implement")
            }

            if location.path() == "/ct/attend/pc" {
                todo!("implement")
            }

            if location.path() == "/attend/tsukuba" {
                todo!("implement")
            }
        }

        // Err(Error::Protocol("todo"))
        Ok(false) // Protocol Error
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

pub fn extract_authenticity_token(html: &str) -> Result<String> {
    todo!("implement")
}
