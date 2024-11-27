use std::future;
use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;
use std::task::{Poll, Waker};
use tokio::time::{Duration, Instant, Sleep};
pub struct HalfSleep(Mutex<HalfSleepInner>);

struct HalfSleepInner {
    sleep: Pin<Box<Sleep>>,
    wakers: Vec<Waker>,
}

impl HalfSleep {
    pub async fn sleep(&self) {
        future::poll_fn(|cx| {
            let mut inner = self.0.lock().unwrap();
            match inner.sleep.as_mut().poll(cx) {
                Poll::Ready(()) => {
                    for waker in inner.wakers.drain(..) {
                        waker.wake();
                    }

                    Poll::Ready(())
                },
                Poll::Pending => {
                    inner.wakers.push(cx.waker().to_owned());
                    Poll::Pending
                },
            }
        })
        .await
    }

    pub fn wake(&self) {
        let mut inner = self.0.lock().unwrap();

        inner.sleep.as_mut().reset(Instant::now());
        for waker in inner.wakers.drain(..) {
            waker.wake();
        }
    }

    pub fn set(&self, duration: Duration) -> &HalfSleep {
        let mut inner = self.0.lock().unwrap();

        inner.sleep.as_mut().reset(Instant::now() + duration);
        for waker in inner.wakers.drain(..) {
            waker.wake();
        }
        self
    }
}

impl Default for HalfSleep {
    fn default() -> Self {
        Self(Mutex::new(HalfSleepInner {
            sleep: Box::pin(tokio::time::sleep(Duration::from_secs(0))),
            wakers: Vec::new(),
        }))
    }
}
