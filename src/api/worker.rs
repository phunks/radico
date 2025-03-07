use crate::api::Api;
use crate::audio::assets::ASSETS;
use crate::audio::player::Player;
use crate::errors::RadicoError::{Forbidden, OperationInterrupted};
use crate::util::menu;
use crate::util::sleep::HalfSleep;
use crate::util::state::StateCollector;
use crate::{lazy_regex, terminal};
use anyhow::{Error, Result};
use chrono::{Local, NaiveDateTime};
use crossterm::event;
use crossterm::event::{poll, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::terminal::enable_raw_mode;
use rand::{distributions::Uniform, prelude::Distribution, thread_rng};
use regex::{Captures, Regex};
use std::collections::VecDeque;
use std::mem;
use std::ops::DerefMut;
use std::sync::{Arc, LazyLock};
use std::sync::atomic::Ordering;
use std::time::Duration;
use log::{error, info, warn};
use tokio::sync::Mutex;
use tokio::time::Instant;

#[derive(Default, Clone)]
pub struct Queue {
    player: Arc<Mutex<Player>>,
    que: Arc<Mutex<VecDeque<Playlist>>>,
    api: Arc<Mutex<Api>>,
    ndt: Arc<std::sync::Mutex<NaiveDateTime>>,
    stat: Arc<Mutex<StateCollector>>,
    s1: Arc<HalfSleep>,
    s2: Arc<HalfSleep>,
    f1: bool,
}

#[derive(Default, Debug, Clone)]
pub struct Playlist {
    url: String,
    buf: Vec<u8>,
}

impl Queue {
    pub async fn worker(&mut self) -> Result<()> {
        player(self.clone()).await?;

        self.api.lock().await.init().await?;
        self.api.lock().await.inquire().await?;
        self.player.lock().await.buffer_clear();

        let mut _delay = Duration::from_secs(5);
        let mut s = self.clone();
        let stat = Arc::clone(&self.stat);

        tokio::spawn(async move {
            loop {
                let a = s.api.lock().await.medialist().await;
                match a {
                    Ok(urls) => {
                        let instant = Instant::now();

                        for url in urls {
                            // TODO value: input contains invalid characters
                            let mut stream_date = match naive_date_from(&url) {
                                Ok(a) => a,
                                Err(e) => {
                                    error!("retry: {:?}\r", e);
                                    naive_date_from(&url).unwrap()
                                }
                            };
                            let last_date = s.ndt.lock().unwrap().to_owned();

                            if last_date < stream_date {
                                #[allow(unused_assignments)]
                                let mut buf = Vec::new();

                                loop {
                                    buf = match s.api.lock().await.get_aac(&url).await {
                                        Ok(buf) => {
                                            if s.f1 {
                                                s.player.lock().await.buffer_clear();
                                                s.f1 = false;
                                            }
                                            buf
                                        },
                                        Err(_e) => {
                                            error!("get_aac error: {:?}\r", _e);
                                            warn!("retry {}\r", &url);
                                            continue;
                                        },
                                    };
                                    break;
                                }

                                s.que.lock().await.push_back(Playlist { url, buf });
                                mem::swap(s.ndt.lock().unwrap().deref_mut(), &mut stream_date);
                            }

                            s.s2.wake();
                        }
                        _delay = s
                            .api
                            .lock()
                            .await
                            .duration(stat.lock().await.delay(), instant)
                            .await;
                    },
                    Err(_) => {
                        terminal::print_error(Error::from(Forbidden));
                        let url =
                            format!("{}{}", "forbidden", Local::now().format("_%Y%m%d_%H%M%S"));
                        let len = s.player.lock().await.buffer_length();
                        if len < 82920 {
                            let p = Playlist {
                                url,
                                buf: ASSETS.get(rand()),
                            };
                            s.que.lock().await.push_back(p);
                            s.f1 = true;
                        }

                        _delay = Duration::from_secs(30);
                        s.s2.wake();
                    },
                };

                s.s1.set(_delay).sleep().await;
            }
        });

        let mut _current_volume = '9';
        enable_raw_mode()?;
        loop {
            if poll(Duration::from_millis(200))? {
                match event::read()? {
                    Event::Key(KeyEvent {
                        code: KeyCode::Char('c'),
                        modifiers: KeyModifiers::CONTROL,
                        ..
                    }) => {
                        terminal::quit(Error::from(OperationInterrupted));
                    },
                    Event::Key(e) => {
                        if e.kind == KeyEventKind::Press {
                            if let KeyCode::Char(c) = e.code {
                                match c {
                                    '0'..='9' => {
                                        self.player.lock().await.volume(c);
                                        _current_volume = c;
                                    },
                                    'n' | 'p' => {
                                        mem::swap(
                                            self.ndt.lock().unwrap().deref_mut(),
                                            &mut NaiveDateTime::default(),
                                        );

                                        self.player.lock().await.buffer_clear();
                                        match c {
                                            'n' => self.api.lock().await.next_station().await?,
                                            'p' => self.api.lock().await.prev_station().await?,
                                            _ => {},
                                        }
                                        self.que.lock().await.clear();
                                        self.s1.wake();
                                        tokio::time::sleep(Duration::from_millis(100)).await;
                                    },
                                    'm' => {
                                        self.api.lock().await.f1.swap(true, Ordering::Relaxed);
                                        terminal::clear_screen();

                                        {
                                            let lock = self.api.lock().await;
                                            println!("{} ({})\r",
                                                     lock.current.area_name.as_ref().unwrap(),
                                                     lock.current.area_id.as_ref().unwrap()
                                            );
                                        }

                                        let stations = self.api.lock().await.get_stations();

                                        let station = match menu::show(&stations) {
                                            Ok(station) => {
                                                let current_station = self
                                                    .api
                                                    .lock()
                                                    .await
                                                    .get_current_station()
                                                    .unwrap();
                                                if current_station == station {
                                                    self.api.lock().await.f1.swap(false, Ordering::Relaxed);
                                                    self.api.lock().await.current_prog().await?;
                                                    continue;
                                                }
                                                station
                                            },
                                            Err(_) => {
                                                self.api.lock().await.f1.swap(false, Ordering::Relaxed);
                                                self.api.lock().await.current_prog().await?;
                                                continue;
                                            },
                                        };
                                        self.api.lock().await.select_station(station).await?;
                                        self.player.lock().await.buffer_clear();
                                        mem::swap(
                                            self.ndt.lock().unwrap().deref_mut(),
                                            &mut NaiveDateTime::default(),
                                        );
                                        self.que.lock().await.clear();
                                        self.s1.wake();
                                        self.api.lock().await.f1.swap(false, Ordering::Relaxed);
                                        self.api.lock().await.current_prog().await?;
                                    },
                                    'i' => self.api.lock().await.current_prog().await?,
                                    _ => {},
                                }
                            }
                        }
                    },
                    _ => {},
                }
            }
        }
    }
}

pub async fn player(medialist: Queue) -> Result<()> {
    let s = medialist.clone();
    tokio::spawn(async move {
        loop {
            let len = s.que.lock().await.len();
            if len > 0 {
                let p = s.que.lock().await.pop_front().unwrap();
                s.player.lock().await.add(&p.buf);

                let ndt = naive_date_from(&p.url).unwrap();
                let blen = s.player.lock().await.buffer_length() as i64;
                s.stat
                    .lock()
                    .await
                    .add(blen, (Local::now().naive_local() - ndt).num_milliseconds());
                info!(
                    "Add {:?} {} {} bytes\r",
                    p.url,
                    len,
                    s.player.lock().await.buffer_length(),
                );
            }

            s.s2.set(Duration::from_secs(5)).sleep().await;
        }
    });

    Ok(())
}

pub fn rand() -> usize {
    let mut rng = thread_rng();
    let between = Uniform::from(0..=5);
    between.sample(&mut rng)
}

lazy_regex!(
    RE: r".*(\d{8})_(\d{6}).*"
);

fn naive_date_from(url: &str) -> Result<NaiveDateTime> {
    let date = RE.replace(url, |caps: &Captures| format!("{}{}", &caps[1], &caps[2]));
    let ndt = NaiveDateTime::parse_from_str(&date, "%Y%m%d%H%M%S")?;
    Ok(ndt)
}
