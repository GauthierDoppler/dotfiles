use starship_battery::units::ratio::percent;
use starship_battery::{Manager, State};

#[derive(Clone, Copy)]
pub struct Battery {
    pub percent: u8,
    pub plugged: bool,
}

impl Battery {
    pub fn read() -> Option<Battery> {
        let battery = Manager::new().ok()?.batteries().ok()?.flatten().next()?;
        Some(Battery {
            percent: battery.state_of_charge().get::<percent>().round().clamp(0.0, 100.0) as u8,
            plugged: !matches!(battery.state(), State::Discharging | State::Empty),
        })
    }

    pub fn icon(self) -> char {
        if self.plugged {
            return '\u{f0084}';
        }
        match self.percent / 10 {
            0 | 1 => '\u{f007a}',
            2 => '\u{f007b}',
            3 => '\u{f007c}',
            4 => '\u{f007d}',
            5 => '\u{f007e}',
            6 => '\u{f007f}',
            7 => '\u{f0080}',
            8 => '\u{f0081}',
            9 => '\u{f0082}',
            _ => '\u{f0079}',
        }
    }
}
