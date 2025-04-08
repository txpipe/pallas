
macro_rules! hashmap {
    // map-like
    ($($k:expr => $v:expr),* $(,)?) => {{
        core::convert::From::from([$(($k, $v),)*])
    }};
    // set-like
    ($($v:expr),* $(,)?) => {{
        core::convert::From::from([$($v,)*])
    }};
}

/// Can be negative
#[derive(Debug, Clone, PartialEq, Eq, Copy, serde::Serialize)]
pub struct ExBudget {
    pub mem: i64,
    pub cpu: i64,
}

impl ExBudget {
    pub fn occurrences(&mut self, n: i64) {
        self.mem *= n;
        self.cpu *= n;
    }

    pub fn max() -> Self {
        ExBudget {
            mem: 14000000000000,
            cpu: 10000000000000,
        }
    }
}

impl Default for ExBudget {
    fn default() -> Self {
        ExBudget {
            mem: 14000000,
            cpu: 10000000000,
        }
    }
}

impl std::ops::Sub for ExBudget {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        ExBudget {
            mem: self.mem - rhs.mem,
            cpu: self.cpu - rhs.cpu,
        }
    }
}