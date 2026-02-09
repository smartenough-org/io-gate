use crate::consts::OutIdx;

/// Internal commands handled by a shutter driver.
#[derive(Eq, PartialEq, Clone, Copy, Debug)]
#[repr(u8)]
pub enum Cmd {
    /// Full analog control: change height and tilt to given values 0-100.
    /// This is a two-step operation: ride (rise or drop) + tilt.
    Go(TargetPosition),

    /// Uncover/open completely. Tilt time + rise time + over_time up.
    Open,
    /// Cover/close completely. Tilt time + drop time + over_time down.
    Close,

    /// Keep height and change tilt to given 0-100.
    Tilt(u8),

    // Tilt helpers.
    /// Tilt(100) - completely closed.
    TiltClose,
    /// Tilt(0) - completely open.
    TiltOpen,
    /// 45 deg.
    TiltHalf,
    /// Open if not completely open; otherwise - close.
    TiltReverse,

    /// Shutters are configured with commands.
    SetIO(/* down */ OutIdx, /* up */ OutIdx),
    // TODO SetRiseDropTime(u16, u16),
    // TODO SetTiltOverTime(u16, u16),
}

mod codes {
    pub const GO: u8 = 0x01;
    pub const OPEN: u8 = 0x02;
    pub const CLOSE: u8 = 0x03;
    pub const TILT: u8 = 0x04;
    pub const TILT_CLOSE: u8 = 0x05;
    pub const TILT_OPEN: u8 = 0x06;
    pub const TILT_HALF: u8 = 0x07;
    pub const TILT_REVERSE: u8 = 0x08;
    pub const SET_IO: u8 = 0x10;
}
impl Cmd {
    pub fn from_raw(raw: &[u8; 5]) -> Option<Self> {
        Some(match raw[0] {
            codes::GO => Cmd::Go(TargetPosition::new(raw[1], raw[2])),
            codes::OPEN => Cmd::Open,
            codes::CLOSE => Cmd::Close,
            codes::TILT => Cmd::Tilt(raw[1]),
            codes::TILT_CLOSE => Cmd::Close,
            codes::TILT_OPEN => Cmd::TiltOpen,
            codes::TILT_HALF => Cmd::TiltHalf,
            codes::TILT_REVERSE => Cmd::TiltReverse,
            codes::SET_IO => Cmd::SetIO(raw[1], raw[2]),
            _ => {
                return None;
            }
        })
    }

    pub fn to_raw(&self, raw: &mut [u8]) {
        raw.fill(0);
        assert!(raw.len() >= 5);
        match self {
            Cmd::Go(position) => {
                raw[0] = codes::GO;
                raw[1] = position.height;
                raw[2] = position.tilt;
            }
            Cmd::Open => {
                raw[0] = codes::OPEN;
            }
            Cmd::Close => {
                raw[0] = codes::CLOSE;
            }
            Cmd::Tilt(tilt) => {
                raw[0] = codes::TILT;
                raw[1] = *tilt;
            }
            Cmd::TiltClose => {
                raw[0] = codes::TILT_CLOSE;
            }
            Cmd::TiltOpen => {
                raw[0] = codes::TILT_OPEN;
            }
            Cmd::TiltHalf => {
                raw[0] = codes::TILT_HALF;
            }
            Cmd::TiltReverse => {
                raw[0] = codes::TILT_REVERSE;
            }
            Cmd::SetIO(down, up) => {
                raw[0] = codes::SET_IO;
                raw[1] = *down;
                raw[2] = *up;
            }
        }
    }
}

/// Current shutter position, or partial position during computation.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Position {
    // Accuracy should allow for 1ms resolution of time. Height 0-100 in 60s
    // would mean 1% takes 600ms. 65535 would have 0.92ms resolution, but we
    // would have to convert. f32 is fine on stm32g4.

    // TODO: Height/Tilt should be an Enum - Known / Guessed. To mark when the position is not synchronized.
    /// Position of shutters. 0 (open) - 100% (closed)
    height: f32,
    /// 0 (open) - 100% (closed)
    tilt: f32,
}

impl Position {
    pub fn new(height: u8, tilt: u8) -> Self {
        assert!(height <= 100);
        assert!(tilt <= 100);
        Self {
            height: height as f32,
            tilt: tilt as f32,
        }
    }

    pub fn new_zero() -> Self {
        Self {
            height: 0.0,
            tilt: 0.0,
        }
    }
}


/// Planned target shutter position.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct TargetPosition {
    // We stick to 0-100% by 1% accuracy.
    /// Position of shutters. 0 (open) - 100% (closed)
    height: u8,
    /// 0 (open) - 100% (closed)
    tilt: u8,
}

impl TargetPosition {
    pub fn new(height: u8, tilt: u8) -> Self {
        Self { height, tilt }
    }

    fn as_position(&self) -> Position {
        Position {
            height: self.height as f32,
            tilt: self.tilt as f32,
        }
    }
}
