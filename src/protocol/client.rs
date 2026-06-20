use std::{sync::Arc, time::Duration};

use reqwest::{
    Url,
    blocking::Client,
    cookie::Jar,
    header::{ACCEPT, ACCEPT_LANGUAGE},
    redirect::Policy,
};

use crate::{diagnostics::Diagnostics, error::Result};

const ATTEND_URL: &str = "https://atmnb.tsukuba.ac.jp/attend/tsukuba";
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
        let response = self.follow.get(attend_url).send()?;
        self.diagnostics.log(format!(
            "GET {ATTEND_URL} -> {} {}",
            response.status(),
            response.url()
        ));
        Ok(false)
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
