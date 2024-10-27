use crate::api::Api;
use crate::args::args_url;
use crate::errors::RadicoError::{OperationInterrupted, Quit};
use crate::player::Player;
use crate::sleep::HalfSleep;
use crate::state::StateCollector;
use crate::{debug_println, terminal};
use anyhow::{Error, Result};
use async_channel::{unbounded, Receiver};
#[allow(unused_imports)]
use chrono::{Local, NaiveDateTime};
use crossterm::{
    event::{self, poll, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    terminal::enable_raw_mode,
};
use futures::future::join_all;
use regex::{Captures, Regex};
use tokio::{
    sync::{Mutex, Notify},
    time::Instant,
};
use std::collections::VecDeque;
use std::default::Default;
use std::sync::{Arc, LazyLock};
use std::time::Duration;

#[derive(Clone, Default)]
struct Queue {
    que: Arc<Mutex<VecDeque<Playlist>>>,
    park: Arc<Notify>,
    stat: Arc<Mutex<StateCollector>>,
    s1: Arc<HalfSleep>,
    s2: Arc<HalfSleep>,
}
#[derive(Default, Debug, Clone)]
struct Playlist {
    url: String,
    buf: Vec<u8>,
}

impl PartialEq for Playlist {
    fn eq(&self, other: &Playlist) -> bool {
        self.url == other.url
    }
}

impl Queue {
    async fn scheduler(&self, srx: Receiver<char>) -> Result<()> {
        let notify = Arc::clone(&self.park);
        let mut _delay = Duration::from_secs(5);

        let url = args_url().to_owned();
        let mut api = Api::new(url);
        api.init().await?;
        api.select().await?;
        api.current_prog().await?;
        let mut last_date = NaiveDateTime::default();
        let stat = Arc::clone(&self.stat);

        loop {
            if let Ok(res) = srx.try_recv() {
                match res {
                    'n' | 'p' => {
                        debug_println!("#1: recv n\r");
                        self.que.lock().await.clear();
                        match res {
                            'n' => api.next_station().await?,
                            'p' => api.prev_station().await?,
                            _ => {},
                        }
                        api.current_prog().await?;
                        last_date = NaiveDateTime::default();
                    },
                    'i' => api.current_prog().await?,
                    'Q' => break,
                    _ => {},
                }
                debug_println!("#1: srx.try_recv end\r");
                _delay = Duration::from_millis(0);
            } else {
                let instant = Instant::now();

                for url in api.medialist().await? {
                    let stream_date = naive_date_from(&url)?;

                    if last_date < stream_date {
                        let buf = api.get_aac(&url).await?;
                        debug_println!("#1: add buf {}\r", url);
                        let p = Playlist { url, buf };
                        self.que.lock().await.push_back(p);
                        last_date = stream_date;
                        notify.notify_one();
                        self.s2.wake();
                    }
                }
                _delay = api.duration(stat.lock().await.delay(), instant).await;
            }
            self.s1.set(_delay).sleep().await;
        }
        Ok(())
    }

    async fn player(&self, prx: Receiver<char>) -> Result<()> {
        let notify = Arc::clone(&self.park);
        notify.notified().await;

        let delay = Duration::from_secs(5);
        let mut player = Player::new();

        let stat = Arc::clone(&self.stat);
        let mut _current_volume = 9;

        loop {
            {
                if let Ok(res) = prx.try_recv() {
                    match res {
                        '0'..='9' => {
                            _current_volume = res.to_string().parse()?;
                            player.volume(_current_volume);
                        },
                        'n' | 'p' => {
                            notify.notified().await;
                            player.buffer_clear();
                            debug_println!("#2: press n\r");
                        },
                        'Q' => break,
                        _ => {},
                    }
                } else {
                    let mut que = self.que.try_lock()?;
                    let qlen = que.len();
                    if qlen != 0 {
                        for _ in 0..qlen {
                            let buf = &que.front().unwrap().buf;
                            player.add(buf);

                            let url = &que.front().unwrap().url;
                            let ndt = naive_date_from(url)?;
                            stat.lock().await.add(player.buffer_length() as i64,
                                (Local::now().naive_local() - ndt).num_milliseconds());
                            debug_println!(
                                "#2: {:?} {} {} bytes\r",
                                url, qlen,
                                player.buffer_length(),
                            );
                            que.drain(0..1);
                        }
                    }
                }
            }
            self.s2.set(delay).sleep().await;
        }
        Ok(())
    }
}

pub async fn main_thread() -> Result<()> {
    let (ptx, prx) = unbounded::<char>();
    let mut hdl = vec![];
    let q = Queue::default();

    let q2 = q.to_owned();
    hdl.push(tokio::spawn(async move { q2.player(prx).await }));

    let (stx, srx) = unbounded::<char>();
    let q1 = q.to_owned();
    hdl.push(tokio::spawn(async move { q1.scheduler(srx).await }));

    let q3 = q.to_owned();
    Arc::clone(&q3.park).notified().await;
    q3.s1.wake();

    loop {
        enable_raw_mode()?;
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
                                'n' | 'p' => {
                                    if stx.try_send(c).is_ok() {
                                        ptx.try_send(c)?;
                                        wake(&q3);
                                    }
                                },
                                'i' => {
                                    stx.try_send(c)?;
                                    wake(&q3);
                                },
                                '0'..='9' => {
                                    ptx.try_send(c)?;
                                    wake(&q3);
                                },
                                'Q' => {
                                    if stx.try_send(c).is_ok() {
                                        ptx.try_send(c)?;
                                        wake(&q3);
                                        break;
                                    }
                                    terminal::quit(Error::from(Quit));
                                },
                                _ => {},
                            }
                            wake(&q3);
                        }
                    }
                }
                _ => {},
            }
        }
    }

    join_all(hdl).await;
    Ok(())
}

fn wake(que: &Queue) {
    que.s1.wake();
    que.s2.wake();
}

static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r".*(\d{8})_(\d{6}).*").unwrap());
pub(crate) fn naive_date_from(url: &str) -> Result<NaiveDateTime> {
    let date = RE.replace(url, |caps: &Captures| format!("{}{}", &caps[1], &caps[2]));
    let ndt = NaiveDateTime::parse_from_str(&date, "%Y%m%d%H%M%S")?;
    Ok(ndt)
}
