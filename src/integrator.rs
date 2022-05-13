use std::ops::{Sub, Mul, Add};

use num::One;



pub struct Integrator<I, A>
where
    A: Into<f64>
{
    alpha: A,
    prev: Option<I>,
}


impl<I, A> Integrator<I, A>
where
    I: Copy + One + Add<I, Output=I> + Mul<I, Output=I> + Sub<A, Output=I> + Mul<A, Output=I>,
    A: Copy + Into<f64>,
{
    pub fn new(alpha: A) -> Self {
        Self { alpha: alpha, prev: None }
    }

    pub fn next(&mut self, x: I) -> Option<I> {
        self.prev = match self.prev {
            Some(p) => Some(p * self.alpha + (I::one() - self.alpha) * x),
            None => Some(x),
        };
        self.prev
    }
}

