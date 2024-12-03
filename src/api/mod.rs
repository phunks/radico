use crate::api::xml::{CurrentProg, PlaylistUrl, Region, Station};
use crate::audio::sink;
use crate::errors::RadicoError::*;
use crate::terminal::args::{usage, Options};
use crate::util::menu::render_config;
use crate::{debug_println, lazy_regex, terminal};
use anyhow::{Context, Error, Result};
use async_recursion::async_recursion;
use base64::engine::general_purpose;
use base64::Engine;
use chrono::{Local, NaiveDateTime};
use http::{
    header::{InvalidHeaderName, InvalidHeaderValue},
    HeaderName,
};
use inquire::{InquireError, Select};
use itertools::Itertools;
use mojimoji_rs::zen_to_han;
use regex::Regex;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client, Response,
};
use serde_xml_rs::from_str;
use std::cmp::PartialEq;
use std::fs::File;
use std::io::Read;
use std::str::FromStr;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, LazyLock};
use std::sync::atomic::Ordering::Relaxed;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::Instant;
use unicode_normalization::UnicodeNormalization;
pub mod worker;
pub mod xml;

pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/69.0.3497.100";

#[derive(Clone)]
pub struct Api {
    pub client: Client,
    pub url: Url,
    pub param: Param,
    pub data: Data,
    pub current: State,
    pub f1: Arc<AtomicBool>,
}

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct Url {
    pub domain: String,
    pub station: Option<String>,
    pub check: Option<String>,
    pub path: Vec<Option<String>>,
    pub prog: Option<String>,
    pub play: Vec<Option<String>>,
}

#[derive(Clone)]
pub struct Data {
    pub region: Region,
}

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct Param {
    pub key: Option<String>,
    pub stations: Vec<String>,
    pub headers: Vec<Kvs>,
    pub station: Vec<Option<String>>,
}

#[derive(Default, Clone)]
pub struct State {
    pub station: Option<Station>,
    pub stations: Vec<Station>,
    station_id: Option<String>,
    area_id: Option<String>,
    area_name: Option<String>,
    plist_url: Option<PlaylistUrl>,
    to: NaiveDateTime,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Kvs {
    pub key: Option<String>,
    pub val: Option<String>,
}

impl PartialEq<Station> for &Station {
    fn eq(&self, other: &Station) -> bool {
        self.name == other.name
    }
}

impl Default for Api {
    fn default() -> Self {
        let mut headers = HeaderMap::new();
        headers.insert("Accept", HeaderValue::from_static("*/*"));
        let arg = Options::init();

        if arg.show_dev_list {
            sink::list_host_devices();
            terminal::quit(Error::from(Quit));
        }

        let cb = Client::builder();
        let c = match &arg.cert {
            None => cb,
            Some(cert) => {
                let mut certs = vec![];
                match File::open(cert) {
                    Ok(mut p) => { p.read_to_end(&mut certs).unwrap() },
                    Err(_) => {
                        println!("certificate error");
                        terminal::quit(Error::from(Quit));
                    },
                };

                cb.add_root_certificate(reqwest::Certificate::from_pem(&certs).unwrap())
            },
        };

        let client = c
            .cookie_store(true)
            .default_headers(headers)
            .connection_verbose(true)
            .gzip(true)
            .timeout(Duration::new(4, 0))
            .user_agent(USER_AGENT)
            .build()
            .unwrap();

        let domain = match arg.url {
            None => {
                println!("{}", usage());
                terminal::quit(Error::from(Quit));
            },
            Some(d) => d,
        };
        Api {
            client,
            url: Url {
                domain,
                ..Default::default()
            },
            param: Default::default(),
            data: Data {
                region: Default::default(),
            },
            current: Default::default(),
            f1: Arc::new(Default::default()),
        }
    }
}

impl Api {
    pub async fn init(&mut self) -> Result<()> {
        self.initializer().await.expect("Initialize Error");
        self.login_check().await.expect("login check error");

        Ok(())
    }

    pub async fn initializer(&mut self) -> Result<()> {
        // top
        let body = self
            .request(self.clone().url.domain.as_str())
            .await
            .with_context(|| format!("Failed to url from {}", &self.url.domain))?;

        let mut top = QuoteUtil::new(&body);
        let menu = &top.find_all("/menu").strip_quote(&EXT_DOUBLE_QUOTE)[0][0];
        let play = &top.find_all("/player").strip_quote(&EXT_DOUBLE_QUOTE)[0][1];
        let js_p = &top.find_all("js-p").strip_quote(&EXT_DOUBLE_QUOTE)[0][0];
        let area = &top.find_all("/area").strip_quote(&EXT_DOUBLE_QUOTE);

        // top-menu
        let body = self
            .request(&format!("{}{}", self.url.domain, menu))
            .await?;

        self.url.check = Some(
            QuoteUtil::new(&body)
                .find_all("/check")
                .strip_quote(&EXT_SINGLE_QUOTE)[0][0]
                .to_owned(),
        );

        // js-p
        let body = self
            .request(&format!("{}{}", self.url.domain, js_p))
            .await?;

        self.param.headers = REG_X_R
            .captures_iter(&body)
            .map(|c| c["x_r"].to_owned())
            .unique()
            .map(|x| Kvs {
                key: Some(x),
                val: None,
            })
            .collect::<Vec<Kvs>>();

        REG_X_VAL
            .captures_iter(&body)
            .map(|c| c.extract())
            .unique()
            .map(|(_, [a, b])| (a.to_owned(), b.to_owned()))
            .for_each(|(a, b)| {
                self.param
                    .headers
                    .iter_mut()
                    .find(|x| x.key == Some(a.to_owned()))
                    .into_iter()
                    .for_each(|x| x.val = Some(b.to_owned()))
            });

        self.url.path = REG_PATH
            .captures_iter(&body)
            .map(|cap| Some(cap["u"].to_owned()))
            .collect::<Vec<_>>();

        let v = REG_TYPE
            .captures_iter(&body)
            .map(|cap| (Some(cap["a"].to_owned()), Some(cap["b"].to_lowercase())))
            .collect::<Vec<_>>()[16]
            .to_owned();

        self.param.station = vec![v.1, v.0];

        // top-play
        let body = self
            .request(&format!("{}{}", self.url.domain, play))
            .await?;

        let param = QuoteUtil::new(&body)
            .find_all("player ")
            .strip_quote(&EXT_SINGLE_QUOTE)[3]
            .to_owned();
        self.param.key = Some(param[1].to_owned());
        self.param.headers[1].val = Some(param[0].to_owned());
        self.param.headers[4].val = param[0].split("_").map(|x| x.to_owned()).next();

        let param = QuoteUtil::new(&body)
            .find_all("/station")
            .strip_quote(&EXT_DOUBLE_QUOTE)[0]
            .to_owned();

        let v = param
            .iter()
            .flat_map(|x| x.split("/"))
            .filter(|x| !x.is_empty())
            .collect::<Vec<_>>();
        self.url.prog = Some([1, 0, 3, 2].iter().map(|&x| v[x]).join("/"));

        let param = QuoteUtil::new(&body)
            .find_all("+ '")
            .strip_quote(&EXT_SINGLE_QUOTE)
            .to_owned();
        let v = param.iter().flat_map(|x| x.to_owned()).collect::<Vec<_>>();
        self.url.play = [2, 3]
            .iter()
            .map(|&x| Some(v[x].to_owned()))
            .collect::<Vec<_>>();

        // current area
        let body = self
            .request(&format!("{}{}", self.url.domain, area[0][0]))
            .await?;

        let areaid = QuoteUtil::new(&body)
            .find_all("/area")
            .strip_quote(&EXT_SINGLE_QUOTE)
            .to_owned();

        let body = self
            .request(&format!("{}?_={}", areaid[0][0], unix_epoch()))
            .await?;

        (self.current.area_id, self.current.area_name) = REG_AREA
            .captures_iter(&body)
            .map(|cap| (Some(cap["a"].to_owned()), Some(cap["b"].to_owned())))
            .collect::<Vec<_>>()[0]
            .to_owned();

        // top-area
        let body = self
            .request(&format!("{}{}", self.url.domain, area[1][0]))
            .await?;

        // full url
        let channel = QuoteUtil::new(&body)
            .find_all("/region")
            .strip_quote(&EXT_SINGLE_QUOTE)[0][0]
            .to_owned();

        let body = self
            .request(&format!("{}{}", self.url.domain, channel))
            .await?;
        let conv = zen_to_han(body, true, true, false);
        self.data.region = from_str(&conv)?;

        Ok(())
    }

    pub async fn next_station(&mut self) -> Result<()> {
        let current = self.to_owned().current.station;
        let mut iter = self.to_owned().current.stations.into_iter().cycle();
        iter.find(|x| x == current.to_owned().unwrap())
            .ok_or(StationError)?;

        self.current.station = Some(iter.next().ok_or(StationError)?);
        self.set_station().await?;
        Ok(())
    }

    pub async fn prev_station(&mut self) -> Result<()> {
        let current = self.to_owned().current.station;
        let mut iter = self.to_owned().current.stations.into_iter().rev().cycle();
        iter.find(|x| x == current.to_owned().unwrap())
            .ok_or(StationError)?;

        self.current.station = Some(iter.next().ok_or(StationError)?);
        self.set_station().await?;
        Ok(())
    }

    pub async fn set_station(&mut self) -> Result<()> {
        let station = self.current.station.to_owned().unwrap();
        self.current.station_id = Some(station.id.to_owned());
        self.current.area_id = Some(station.area_id.to_owned());

        debug_println!("{:?}\r", station);

        self.playlist_url().await.expect("failed to get playlist");
        self.station_url().await.expect("failed to get station url");
        self.current_prog().await?;
        debug_println!(">>>>> set {:?}\r", station);
        Ok(())
    }

    pub async fn inquire(&mut self) -> Result<()> {
        inquire::set_global_render_config(render_config());

        let area_id = self.current.area_id.as_ref().unwrap();
        let area_name = self.current.area_name.as_ref().unwrap();
        println!("{} ({})\r", area_name, area_id);
        let stations = self
            .to_owned()
            .data
            .region
            .stations
            .into_iter()
            .flat_map(|x| x.station.into_iter().collect::<Vec<_>>())
            .collect::<Vec<_>>();

        self.set_stations(&stations).expect("failed to set station");
        let v = stations
            .iter()
            .map(|x| x.name.to_owned())
            .collect::<Vec<_>>();

        loop {
            match Select::new("station?", v.to_owned()).prompt() {
                Ok(station) => {
                    self.param.stations = v;
                    self.current.station = Some(
                        stations
                            .iter()
                            .find(|x| x.name == station)
                            .ok_or(InquireError)?
                            .to_owned(),
                    );
                    break;
                },
                Err(e) => match e {
                    InquireError::OperationCanceled => return Err(Error::from(Cancel)),
                    InquireError::OperationInterrupted => {
                        return Err(Error::from(OperationInterrupted))
                    },
                    _ => continue,
                },
            };
        }

        self.set_station().await?;
        Ok(())
    }

    pub async fn select_station(&mut self, station: String) -> Result<()> {
        self.current.station = Some(
            self.current
                .stations
                .iter()
                .find(|x| x.name == station)
                .ok_or(InquireError)?
                .to_owned(),
        );

        debug_println!(">>>>> {:?}\r", station);
        self.set_station().await?;
        Ok(())
    }

    pub fn get_stations(&self) -> Vec<String> {
        self.param.stations.to_vec()
    }

    pub fn get_current_station(&self) -> Option<String> {
        Some(self.clone().current.station.unwrap().name)
    }

    fn set_stations(&mut self, v: &Vec<Station>) -> Result<()> {
        self.current.stations = v.to_owned();
        Ok(())
    }

    async fn login_check(&mut self) -> Result<()> {
        match &self.url.check {
            None => {},
            Some(check) => {
                self.backoff_request(&format!("{}{}", self.url.domain, check), None)
                    .await?;
            },
        }
        Ok(())
    }

    pub async fn playlist_url(&mut self) -> Result<()> {
        let res = self
            .backoff_request(
                &format!(
                    "{}{}{}/{}.xml",
                    self.url.domain,
                    self.url.path[2].to_owned().unwrap(),
                    self.param.headers[1].val.to_owned().unwrap(),
                    self.current.station_id.to_owned().unwrap()
                ),
                None,
            )
            .await?;

        let body = res.text().await?;
        self.current.plist_url = Some(from_str(&body)?);

        Ok(())
    }

    async fn get_auth_token(&mut self) -> Result<HeaderMap> {
        let auth_token = self.auth_token().await?;
        let mut headers = HeaderMap::new();
        headers.insert(
            self.key(13)?,
            self.current.area_id.to_owned().unwrap().parse()?,
        );

        headers.insert(self.key(5)?, auth_token.parse()?);
        Ok(headers)
    }

    pub async fn station_url(&mut self) -> Result<()> {
        let hash = gen_hash_key();

        let station = self.current.station_id.to_owned().ok_or(StationError)?;
        let playlist = self.current.plist_url.to_owned().ok_or(PlaylistError)?;

        let v = self.to_owned().url.play;
        let w = self.to_owned().param.station;
        let header = self.get_auth_token().await?;
        let res = match self
            .backoff_request(
                &format!(
                    "{}{}{}{}&{}={}&{}=b",
                    &playlist.url[0].value,
                    v[0].to_owned().unwrap(),
                    station,
                    v[1].to_owned().unwrap(),
                    w[0].to_owned().unwrap(),
                    hash,
                    w[1].to_owned().unwrap()
                ),
                Some(header),
            )
            .await
        {
            Ok(res) => res,
            Err(e) => return Err(e),
        };

        match res.text().await {
            Ok(list) => {
                if list == "forbidden" {
                    self.url.station = None;
                }
                self.url.station = list
                    .split("\n")
                    .filter(|x| x.contains("https://"))
                    .map(|x| x.to_string())
                    .next();
            },
            Err(e) => terminal::quit(Error::from(e)),
        }

        Ok(())
    }

    pub async fn medialist(&mut self) -> Result<Vec<String>> {
        match self.clone().url.station {
            None => Err(Error::from(Forbidden)),
            Some(url) => {
                let res = match self.backoff_request(&url, None).await {
                    Ok(res) => res,
                    Err(e) => return Err(e),
                };

                let body = res.text().await.expect("medialist response error");
                Ok(body
                    .split("\n")
                    .filter(|x| x.contains("https://"))
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>())
            },
        }
    }

    pub async fn get_aac(&mut self, url: &str) -> Result<Vec<u8>> {
        match self.backoff_request(url, None).await {
            Ok(r) => Ok(r.bytes().await?.to_vec()),
            Err(e) => Err(e),
        }
    }

    async fn request(&mut self, url: &str) -> Result<String> {
        let res = self.backoff_request(url, None).await?;
        let body = res.text().await?;
        Ok(body)
    }

    #[async_recursion]
    async fn backoff_request(&mut self, url: &str, header: Option<HeaderMap>) -> Result<Response> {
        let mut count = 0_i8;

        let client = match header {
            None => self.client.get(url),
            Some(header) => self.client.get(url).headers(header),
        };

        loop {
            let res = match client.try_clone() {
                None => {
                    debug_println!("client error\r");
                    return Err(Error::from(ClientError));
                },
                Some(client) => client.send().await,
            };

            match res {
                Ok(res) => match res.status().as_u16() {
                    200..=299 => {
                        debug_println!("Fetching {} {}\r", res.status(), url);
                        return Ok(res);
                    },
                    400 => {
                        debug_println!("bad request {} {}\r", res.status(), url);
                        return Ok(res);
                    },
                    403 => {
                        debug_println!("forbidden {} {}\r", res.status(), url);
                        return Ok(res);
                    },
                    401..=499 => {
                        debug_println!("client error {} {}\r", res.status(), url);
                        self.station_url().await?;
                    },
                    500..=599 => {
                        debug_println!("server error {} {}\r", res.status(), url);
                        self.station_url().await?;
                    },
                    _ => {
                        debug_println!("other {} {}\r", res.status(), url);
                        self.station_url().await?;
                    },
                },
                Err(e) => {
                    debug_println!("fetch error: {}\r", e);
                    if count > 1 {
                        return Err(Error::from(e));
                    }
                },
            };
            if count > 1 {
                return Err(Error::from(ClientError));
            }
            count += 1;
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }

    async fn auth_token(&mut self) -> Result<String> {
        let mut headers = HeaderMap::new();

        for i in 1..5 {
            headers.insert(self.key(i)?, self.val(i)?);
        }

        let url = format!(
            "{}{}",
            self.url.domain,
            &self.url.path[0].to_owned().unwrap()
        );

        let res = self.backoff_request(&url, Some(headers)).await?;
        let auth_token = res
            .headers()
            .get(self.key(5).unwrap())
            .ok_or(AuthError)?
            .to_str()?;
        let offset = res
            .headers()
            .get(self.key(10)?)
            .ok_or(AuthError)?
            .to_str()?
            .parse::<usize>()?;
        let length = res
            .headers()
            .get(self.key(11)?)
            .ok_or(AuthError)?
            .to_str()?
            .parse::<usize>()?;

        let key = self.to_owned().param.key.unwrap();
        let partial_key = general_purpose::STANDARD.encode(&key[offset..offset + length]);
        self.param.headers[5].val = Some(auth_token.parse()?);
        self.param.headers[6].val = Some(partial_key);

        let url = format!(
            "{}{}",
            self.url.domain,
            &self.url.path[1].to_owned().unwrap()
        );
        headers = HeaderMap::new();
        for i in 3..7 {
            headers.insert(self.key(i)?, self.val(i)?);
        }

        self.backoff_request(&url, Some(headers)).await?;
        Ok(auth_token.to_string())
    }

    pub async fn current_prog(&mut self) -> Result<()> {
        if self.f1.load(Relaxed) { return Ok(()) }
        let l = (Local::now() - Duration::from_secs(18000))
            .format("%Y%m%d")
            .to_string();

        let res = self
            .backoff_request(
                &format!(
                    "{}/{}/{}/{}.xml",
                    self.url.domain,
                    self.to_owned().url.prog.unwrap(),
                    l,
                    &self.current.station_id.to_owned().unwrap()
                ),
                None,
            )
            .await?;
        let body = res.text().await?;
        let station = &self.to_owned().current.station.unwrap().name;
        let current: CurrentProg = match from_str(&body) {
            Ok(a) => a,
            Err(e) => {
                debug_println!("{:?}", e);
                return Err(Error::from(e));
            },
        };
        if let Some(i) = current.stations.station.progs.prog.iter().rev().find(|x| {
            NaiveDateTime::parse_from_str(&x.ft, "%Y%m%d%H%M%S").unwrap()
                < Local::now().naive_local()
        }) {
            terminal::clear_screen();
            self.current.to = NaiveDateTime::parse_from_str(&i.to, "%Y%m%d%H%M%S")?;

            println!(
                "{}\n\r{} - {} {}\n\r{}\r",
                station,
                NaiveDateTime::parse_from_str(&i.ft, "%Y%m%d%H%M%S")
                    .unwrap()
                    .format("%H:%M"),
                self.current.to.format("%H:%M"),
                i.title,
                strip_html(&i.info).trim()
            );
        }

        Ok(())
    }

    pub async fn duration(&mut self, ave: Duration, instant: Instant) -> Duration {
        let local = Local::now().naive_local();

        let prog_end = (self.current.to - (local - ave)).num_milliseconds();
        let mut delay = Duration::from_secs(5);
        debug_println!("{:?} {:?} {:?}\r", local, ave, self.current.to);
        if let 0..=5000 = prog_end {
            delay = Duration::from_millis(prog_end as u64);
        } else if local - ave > self.current.to {
            self.current_prog().await.unwrap();
        } else {
            delay = sleep(instant.elapsed());
        }
        delay
    }

    fn key(&self, n: usize) -> core::result::Result<HeaderName, InvalidHeaderName> {
        HeaderName::from_str(&self.param.headers[n].to_owned().key.unwrap())
    }
    fn val(&self, n: usize) -> core::result::Result<HeaderValue, InvalidHeaderValue> {
        HeaderValue::from_str(&self.param.headers[n].to_owned().val.unwrap())
    }
}

fn sleep(elapsed: Duration) -> Duration {
    let mut delay = Duration::from_secs(5);
    if Duration::new(5, 0) > elapsed {
        delay = Duration::from_secs(5) - elapsed;
    }
    delay
}

fn strip_html(source: &str) -> String {
    let result = REG_CONDENSE
        .replace_all(source, " ")
        .cjk_compat_variants()
        .collect::<String>();
    let source = result.replace(r"\n", r"\n\r");

    let mut data = String::new();
    let mut inside = false;

    for c in source.chars() {
        if c == '<' {
            inside = true;
            continue;
        }
        if c == '>' {
            inside = false;
            continue;
        }
        if !inside {
            data.push(c);
        }
    }
    // data
    zen_to_han(data, true, true, false)
}

fn gen_hash_key() -> String {
    let digest = md5::compute(b"abcdefghijklmnopqrstuvwxyz");
    format!("{:x}", digest)
}

pub fn unix_epoch() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

lazy_regex!(
    EXT_DOUBLE_QUOTE: r#""(.*?)""#,
    EXT_SINGLE_QUOTE: r#"'(.*?)'"#,
    REG_PATH:         r#"host\+"(?<u>[^"]+)"#,
    REG_X_R:          r#""(?<x_r>(?i)X-R.*?)""#,
    REG_X_VAL:        r#""((?i)X-R[^"]+)":"([^"]+)""#,
    REG_CONDENSE:     r"\s+",
    REG_TYPE:         r#"(?<a>typ.),"(?<b>.*?)""#,
    REG_AREA:         r#".*?"(?<a>.*)?">(?<b>.*?)<.*"#
);

#[derive(Clone, Default)]
pub struct QuoteUtil<'a> {
    txt: &'a str,
    res: Vec<&'a str>,
}

impl<'a> QuoteUtil<'a> {
    fn new(txt: &'a str) -> Self {
        QuoteUtil { txt, res: vec![] }
    }

    fn find_all(&mut self, key: &'a str) -> Self {
        let res = self
            .txt
            .split("\n")
            .filter(|&x| x.contains(key))
            .collect::<Vec<_>>();
        QuoteUtil {
            res,
            ..Default::default()
        }
    }

    // Extract string from between quotations double quote
    fn strip_quote(&self, r: &LazyLock<Regex>) -> Vec<Vec<String>> {
        let mut v = vec![];
        for i in &self.res {
            v.push(
                r.captures_iter(i)
                    .map(|cap| cap[1].to_owned())
                    .collect::<Vec<_>>(),
            );
        }
        v
    }
}
