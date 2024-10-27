use std::time::Duration;
use crate::debug_println;

#[derive(Default)]
pub struct StateCollector {
    v: Box<[(i64, i64); 4]>,
}

impl StateCollector {
    pub fn add(&mut self, a: i64, b: i64) {
        self.v.rotate_left(1);
        let l = self.v.len() - 1;
        let _ = std::mem::replace(&mut self.v[l], (a, b));
    }

    #[allow(dead_code)]
    pub fn reset(&mut self, a: i64, b: i64) {
        let _ = std::mem::replace(&mut self.v, Box::new([(a, b); 4]));
    }

    pub fn delay(&self) -> Duration {
        let (a, b): (Vec<i64>, Vec<i64>) = self.v.iter().cloned().unzip();
        debug_println!(">>> {:?} {:?}\r", a, b);
        Duration::from_millis(
            ((a.iter().sum::<i64>() as f64 / 24.8_f64) // 5 sec 31k
                + (b.iter().sum::<i64>() as f64 / 4_f64) - 5300_f64) as u64, // adjust
        )
    }
}
