use crate::Position;

const MAX_SHIFT: u8 = 15;

/// Suavizado exponencial entero: `y += (x - y) / 2^shift`, redondeado alejándose de cero
/// para converger exactamente al objetivo. `shift` se limita a 15; 0 deja pasar la señal.
#[derive(Clone, Copy, Debug)]
pub struct Filter {
    shift: u8,
    state: Option<i32>,
}

impl Filter {
    pub fn new(shift: u8) -> Self {
        Self { shift: shift.min(MAX_SHIFT), state: None }
    }

    pub fn apply(&mut self, input: Position) -> Position {
        let x = i32::from(input.get());
        let y = match self.state {
            None => x,
            Some(y) => {
                let d = x - y;
                let step = if d > 0 { (d + (1 << self.shift) - 1) >> self.shift } else { d >> self.shift };
                y + step
            }
        };
        self.state = Some(y);
        Position::saturating(y as u16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(v: u16) -> Position {
        Position::saturating(v)
    }

    #[test]
    fn first_sample_seeds_state() {
        let mut f = Filter::new(3);
        assert_eq!(f.apply(pos(700)), pos(700));
    }

    #[test]
    fn shift_zero_is_passthrough() {
        let mut f = Filter::new(0);
        for v in [0, 1000, 3, 999, 500] {
            assert_eq!(f.apply(pos(v)), pos(v));
        }
    }

    #[test]
    fn rising_step_is_monotone_and_converges() {
        let mut f = Filter::new(3);
        f.apply(pos(0));
        let mut prev = 0;
        for _ in 0..200 {
            let y = f.apply(pos(1000)).get();
            assert!(y >= prev && y <= 1000);
            prev = y;
        }
        assert_eq!(prev, 1000);
    }

    #[test]
    fn falling_step_is_monotone_and_converges() {
        let mut f = Filter::new(3);
        f.apply(pos(1000));
        let mut prev = 1000;
        for _ in 0..200 {
            let y = f.apply(pos(0)).get();
            assert!(y <= prev);
            prev = y;
        }
        assert_eq!(prev, 0);
    }

    #[test]
    fn rising_and_falling_converge_to_the_same_target() {
        let mut up = Filter::new(4);
        let mut down = Filter::new(4);
        up.apply(pos(0));
        down.apply(pos(1000));
        for _ in 0..300 {
            up.apply(pos(500));
            down.apply(pos(500));
        }
        assert_eq!(up.apply(pos(500)), pos(500));
        assert_eq!(down.apply(pos(500)), pos(500));
    }

    #[test]
    fn oversized_shift_does_not_overflow_and_still_converges() {
        let mut f = Filter::new(u8::MAX);
        f.apply(pos(0));
        for _ in 0..2000 {
            f.apply(pos(Position::MAX));
        }
        assert_eq!(f.apply(pos(Position::MAX)), pos(Position::MAX));
    }

    #[test]
    fn full_range_swing_stays_in_range() {
        let mut f = Filter::new(1);
        for i in 0..100u16 {
            let y = f.apply(pos(if i % 2 == 0 { 0 } else { Position::MAX })).get();
            assert!(y <= Position::MAX);
        }
    }
}
