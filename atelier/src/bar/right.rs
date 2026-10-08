use chrono::NaiveDateTime;
use unicode_width::UnicodeWidthStr;

use crate::bar::battery::Battery;
use crate::git::RepoCounts;

const KEYS: &str = "#[fg=#8caaee,bold]";
const INSERTIONS: &str = "#[fg=#eebebe,nobold]";
const DELETIONS: &str = "#[fg=#ea999c,nobold]";
const SYNC: &str = "#[fg=#99d1db,nobold]";
const BATTERY: &str = "#[fg=#81c8be,nobold]";
const LOW_BATTERY: &str = "#[fg=#e78284,bold]";
const DATE: &str = "#[fg=#949cbb,nobold]";
const CLOCK: &str = "#[fg=#c6d0f5,bold]";
const END: &str = "#[nobold]  ";
const DATE_FORMAT: &str = "%a %d %b";

const KEYS_WIDTH: usize = 10;
const END_WIDTH: usize = 2;

pub struct Right<'a> {
    pub key_table: &'a str,
    pub repo: RepoCounts,
    pub battery: Option<Battery>,
    pub now: NaiveDateTime,
}

#[derive(Clone, Copy)]
enum Segment {
    Repo,
    Battery,
    Date,
    Clock,
}

impl Segment {
    fn gap(self) -> usize {
        match self {
            Segment::Repo => 0,
            Segment::Battery | Segment::Date => 4,
            Segment::Clock => 3,
        }
    }

    fn width(self) -> usize {
        match self {
            Segment::Repo => 15,
            Segment::Battery => 6,
            Segment::Date => 10,
            Segment::Clock => 5,
        }
    }
}

fn segments(client_width: u16, has_battery: bool) -> Vec<Segment> {
    let tier: &[Segment] = match client_width {
        120.. => &[
            Segment::Repo,
            Segment::Battery,
            Segment::Date,
            Segment::Clock,
        ],
        100..=119 => &[Segment::Repo, Segment::Battery, Segment::Clock],
        90..=99 => &[Segment::Repo, Segment::Clock],
        _ => &[Segment::Clock],
    };
    tier.iter()
        .copied()
        .filter(|segment| has_battery || !matches!(segment, Segment::Battery))
        .collect()
}

pub fn width(client_width: u16, has_battery: bool) -> usize {
    segments(client_width, has_battery)
        .into_iter()
        .map(|segment| segment.gap() + segment.width())
        .sum::<usize>()
        + KEYS_WIDTH
        + END_WIDTH
}

impl Right<'_> {
    pub fn render(&self, client_width: u16) -> String {
        let keys = if self.key_table == "root" {
            ""
        } else {
            self.key_table
        };
        format!(
            "{KEYS}{}{}{}{END}",
            pad_right(keys, KEYS_WIDTH),
            body(self.repo, self.battery, client_width, Some(self.now)),
            self.now.format("%H:%M")
        )
    }
}

pub fn pushed(repo: RepoCounts, battery: Option<Battery>, client_width: u16) -> String {
    body(repo, battery, client_width, None)
}

fn body(
    repo: RepoCounts,
    battery: Option<Battery>,
    client_width: u16,
    now: Option<NaiveDateTime>,
) -> String {
    let mut out = String::new();
    for segment in segments(client_width, battery.is_some()) {
        out.push_str(&" ".repeat(segment.gap()));
        match segment {
            Segment::Repo => out.push_str(&repo_segment(repo)),
            Segment::Battery => {
                if let Some(battery) = battery {
                    let text = battery_segment(battery);
                    match now {
                        Some(_) => out.push_str(&text),
                        None => out.push_str(&text.replace('%', "%%")),
                    }
                }
            }
            Segment::Date => {
                out.push_str(DATE);
                match now {
                    Some(now) => out.push_str(&now.format(DATE_FORMAT).to_string()),
                    None => out.push_str(DATE_FORMAT),
                }
            }
            Segment::Clock => out.push_str(CLOCK),
        }
    }
    out
}

fn repo_segment(repo: RepoCounts) -> String {
    let parts: Vec<(&str, String)> = [
        (INSERTIONS, '+', repo.insertions),
        (DELETIONS, '−', repo.deletions),
        (SYNC, '↑', repo.ahead),
        (SYNC, '↓', repo.behind),
    ]
    .into_iter()
    .filter(|part| part.2 > 0)
    .map(|(style, glyph, count)| (style, format!("{glyph}{}", abbreviated(count))))
    .collect();
    let room = Segment::Repo.width();
    let width = |kept: usize| {
        let shown: usize = parts[..kept].iter().map(|(_, text)| text.width() + 1).sum();
        let ellipsis = if kept < parts.len() { 2 } else { 0 };
        (shown + ellipsis).saturating_sub(1)
    };
    let kept = (0..=parts.len())
        .rev()
        .find(|&kept| width(kept) <= room)
        .unwrap_or(0);
    let mut words: Vec<String> = parts[..kept]
        .iter()
        .map(|(style, text)| format!("{style}{text}"))
        .collect();
    if let Some((style, _)) = parts.get(kept) {
        words.push(format!("{style}…"));
    }
    let used = width(kept);
    let body = words.join(" ");
    let mut out = " ".repeat(room.saturating_sub(used));
    out.push_str(&body);
    out
}

fn abbreviated(count: u64) -> String {
    match count {
        0..=999 => count.to_string(),
        1_000..=999_999 => format!("{}k", count / 1_000),
        _ => format!("{}M", count / 1_000_000),
    }
}

fn battery_segment(battery: Battery) -> String {
    let style = if !battery.plugged && battery.percent < 20 {
        LOW_BATTERY
    } else {
        BATTERY
    };
    let text = format!("{} {}%", battery.icon(), battery.percent);
    format!("{style}{}", pad_right(&text, Segment::Battery.width()))
}

fn pad_right(text: &str, cells: usize) -> String {
    format!("{text}{}", " ".repeat(cells.saturating_sub(text.width())))
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;

    const CLEAN: RepoCounts = RepoCounts {
        insertions: 0,
        deletions: 0,
        ahead: 0,
        behind: 0,
    };
    const EDITS: RepoCounts = RepoCounts {
        insertions: 141,
        deletions: 111,
        ahead: 0,
        behind: 0,
    };
    const AHEAD: RepoCounts = RepoCounts {
        insertions: 0,
        deletions: 0,
        ahead: 2,
        behind: 0,
    };
    const BOTH: RepoCounts = RepoCounts {
        insertions: 111,
        deletions: 113,
        ahead: 2,
        behind: 1,
    };

    const LARGE: RepoCounts = RepoCounts {
        insertions: 1234,
        deletions: 5678,
        ahead: 12,
        behind: 3,
    };
    const HUGE: RepoCounts = RepoCounts {
        insertions: 2_345_678,
        deletions: 98_765,
        ahead: 1234,
        behind: 999,
    };

    const fn discharging(percent: u8) -> Battery {
        Battery {
            percent,
            plugged: false,
        }
    }

    const fn plugged(percent: u8) -> Battery {
        Battery {
            percent,
            plugged: true,
        }
    }

    fn tuesday_4_august() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 8, 4)
            .unwrap()
            .and_hms_opt(9, 5, 0)
            .unwrap()
    }

    fn right(key_table: &str, repo: RepoCounts, battery: Option<Battery>) -> Right<'_> {
        Right {
            key_table,
            repo,
            battery,
            now: tuesday_4_august(),
        }
    }

    fn cells(rendered: &str) -> usize {
        let mut visible = String::new();
        let mut rest = rendered;
        while let Some(start) = rest.find("#[") {
            visible.push_str(&rest[..start]);
            rest = &rest[start..];
            rest = &rest[rest.find(']').unwrap() + 1..];
        }
        visible.push_str(rest);
        visible.chars().count()
    }

    type BashRow = (RepoCounts, Option<Battery>, u16, &'static str, usize, &'static str);

    #[rustfmt::skip]
    const BASH: &[BashRow] = &[
        (CLEAN, Some(discharging(87)), 80, "root", 20, "#[fg=#8caaee,bold]             #[fg=#c6d0f5,bold]09:05#[nobold]  "),
        (CLEAN, Some(discharging(87)), 100, "root", 45, "#[fg=#8caaee,bold]                             #[fg=#81c8be,nobold]\u{f0081} 87%    #[fg=#c6d0f5,bold]09:05#[nobold]  "),
        (CLEAN, Some(discharging(87)), 120, "root", 59, "#[fg=#8caaee,bold]                             #[fg=#81c8be,nobold]\u{f0081} 87%     #[fg=#949cbb,nobold]Tue 04 Aug   #[fg=#c6d0f5,bold]09:05#[nobold]  "),
        (CLEAN, Some(discharging(87)), 120, "prefix", 59, "#[fg=#8caaee,bold]prefix                       #[fg=#81c8be,nobold]\u{f0081} 87%     #[fg=#949cbb,nobold]Tue 04 Aug   #[fg=#c6d0f5,bold]09:05#[nobold]  "),
        (CLEAN, Some(discharging(87)), 200, "root", 59, "#[fg=#8caaee,bold]                             #[fg=#81c8be,nobold]\u{f0081} 87%     #[fg=#949cbb,nobold]Tue 04 Aug   #[fg=#c6d0f5,bold]09:05#[nobold]  "),
        (EDITS, Some(plugged(64)), 80, "root", 20, "#[fg=#8caaee,bold]             #[fg=#c6d0f5,bold]09:05#[nobold]  "),
        (EDITS, Some(plugged(64)), 100, "root", 45, "#[fg=#8caaee,bold]                #[fg=#eebebe,nobold]+141 #[fg=#ea999c,nobold]−111    #[fg=#81c8be,nobold]\u{f0084} 64%    #[fg=#c6d0f5,bold]09:05#[nobold]  "),
        (EDITS, Some(plugged(64)), 120, "root", 59, "#[fg=#8caaee,bold]                #[fg=#eebebe,nobold]+141 #[fg=#ea999c,nobold]−111    #[fg=#81c8be,nobold]\u{f0084} 64%     #[fg=#949cbb,nobold]Tue 04 Aug   #[fg=#c6d0f5,bold]09:05#[nobold]  "),
        (EDITS, Some(plugged(64)), 120, "prefix", 59, "#[fg=#8caaee,bold]prefix          #[fg=#eebebe,nobold]+141 #[fg=#ea999c,nobold]−111    #[fg=#81c8be,nobold]\u{f0084} 64%     #[fg=#949cbb,nobold]Tue 04 Aug   #[fg=#c6d0f5,bold]09:05#[nobold]  "),
        (EDITS, Some(plugged(64)), 200, "root", 59, "#[fg=#8caaee,bold]                #[fg=#eebebe,nobold]+141 #[fg=#ea999c,nobold]−111    #[fg=#81c8be,nobold]\u{f0084} 64%     #[fg=#949cbb,nobold]Tue 04 Aug   #[fg=#c6d0f5,bold]09:05#[nobold]  "),
        (AHEAD, Some(discharging(15)), 80, "root", 20, "#[fg=#8caaee,bold]             #[fg=#c6d0f5,bold]09:05#[nobold]  "),
        (AHEAD, Some(discharging(15)), 100, "root", 45, "#[fg=#8caaee,bold]                       #[fg=#99d1db,nobold]↑2    #[fg=#e78284,bold]\u{f007a} 15%    #[fg=#c6d0f5,bold]09:05#[nobold]  "),
        (AHEAD, Some(discharging(15)), 120, "root", 59, "#[fg=#8caaee,bold]                       #[fg=#99d1db,nobold]↑2    #[fg=#e78284,bold]\u{f007a} 15%     #[fg=#949cbb,nobold]Tue 04 Aug   #[fg=#c6d0f5,bold]09:05#[nobold]  "),
        (AHEAD, Some(discharging(15)), 120, "prefix", 59, "#[fg=#8caaee,bold]prefix                 #[fg=#99d1db,nobold]↑2    #[fg=#e78284,bold]\u{f007a} 15%     #[fg=#949cbb,nobold]Tue 04 Aug   #[fg=#c6d0f5,bold]09:05#[nobold]  "),
        (AHEAD, Some(discharging(15)), 200, "root", 59, "#[fg=#8caaee,bold]                       #[fg=#99d1db,nobold]↑2    #[fg=#e78284,bold]\u{f007a} 15%     #[fg=#949cbb,nobold]Tue 04 Aug   #[fg=#c6d0f5,bold]09:05#[nobold]  "),
        (BOTH, Some(discharging(87)), 80, "root", 20, "#[fg=#8caaee,bold]             #[fg=#c6d0f5,bold]09:05#[nobold]  "),
        (BOTH, Some(discharging(87)), 100, "root", 45, "#[fg=#8caaee,bold]          #[fg=#eebebe,nobold]+111 #[fg=#ea999c,nobold]−113 #[fg=#99d1db,nobold]↑2 #[fg=#99d1db,nobold]↓1    #[fg=#81c8be,nobold]\u{f0081} 87%    #[fg=#c6d0f5,bold]09:05#[nobold]  "),
        (BOTH, Some(discharging(87)), 120, "root", 59, "#[fg=#8caaee,bold]          #[fg=#eebebe,nobold]+111 #[fg=#ea999c,nobold]−113 #[fg=#99d1db,nobold]↑2 #[fg=#99d1db,nobold]↓1    #[fg=#81c8be,nobold]\u{f0081} 87%     #[fg=#949cbb,nobold]Tue 04 Aug   #[fg=#c6d0f5,bold]09:05#[nobold]  "),
        (BOTH, Some(discharging(87)), 120, "prefix", 59, "#[fg=#8caaee,bold]prefix    #[fg=#eebebe,nobold]+111 #[fg=#ea999c,nobold]−113 #[fg=#99d1db,nobold]↑2 #[fg=#99d1db,nobold]↓1    #[fg=#81c8be,nobold]\u{f0081} 87%     #[fg=#949cbb,nobold]Tue 04 Aug   #[fg=#c6d0f5,bold]09:05#[nobold]  "),
        (BOTH, Some(discharging(87)), 200, "root", 59, "#[fg=#8caaee,bold]          #[fg=#eebebe,nobold]+111 #[fg=#ea999c,nobold]−113 #[fg=#99d1db,nobold]↑2 #[fg=#99d1db,nobold]↓1    #[fg=#81c8be,nobold]\u{f0081} 87%     #[fg=#949cbb,nobold]Tue 04 Aug   #[fg=#c6d0f5,bold]09:05#[nobold]  "),
    ];

    #[test]
    fn renders_exactly_what_the_bash_script_it_replaced_rendered() {
        for &(repo, battery, client_width, key_table, bash_width, bash) in BASH {
            let right = right(key_table, repo, battery);
            assert_eq!(right.render(client_width), bash, "at {client_width} columns");
            assert_eq!(width(client_width, true), bash_width, "at {client_width} columns");
        }
    }

    const TIERS: [u16; 6] = [80, 90, 99, 100, 120, 200];

    fn snapshot_tiers(name: &str, right: &Right) {
        let rendered: Vec<String> = TIERS
            .iter()
            .map(|&client_width| format!("{client_width:>3} |{}|", right.render(client_width)))
            .collect();
        insta::assert_snapshot!(name, rendered.join("\n"));
    }

    #[test]
    fn every_tier_with_repo_counts_and_battery() {
        snapshot_tiers("counts_and_battery", &right("root", BOTH, Some(discharging(87))));
    }

    #[test]
    fn every_tier_with_repo_counts_and_no_battery() {
        snapshot_tiers("counts_no_battery", &right("root", BOTH, None));
    }

    #[test]
    fn every_tier_without_repo_counts_with_battery() {
        snapshot_tiers("no_counts_battery", &right("root", CLEAN, Some(plugged(100))));
    }

    #[test]
    fn every_tier_without_repo_counts_or_battery() {
        snapshot_tiers("no_counts_no_battery", &right("prefix", CLEAN, None));
    }

    #[test]
    fn every_tier_with_counts_too_wide_for_their_segment() {
        snapshot_tiers("large_counts", &right("root", LARGE, Some(discharging(87))));
        snapshot_tiers("huge_counts", &right("root", HUGE, None));
    }

    #[test]
    fn counts_of_a_thousand_and_more_are_abbreviated() {
        let segment = |repo| {
            let rendered = repo_segment(repo);
            let mut visible = String::new();
            let mut rest = rendered.as_str();
            while let Some(start) = rest.find("#[") {
                visible.push_str(&rest[..start]);
                rest = &rest[rest.find(']').unwrap() + 1..];
            }
            visible.push_str(rest);
            visible.trim_start().to_string()
        };
        assert_eq!(segment(BOTH), "+111 −113 ↑2 ↓1");
        assert_eq!(segment(LARGE), "+1k −5k ↑12 ↓3");
        assert_eq!(segment(HUGE), "+2M −98k ↑1k …");
    }

    #[test]
    fn a_machine_without_a_battery_drops_the_segment_and_its_gap() {
        assert_eq!(width(200, false), 49);
        assert_eq!(width(100, false), 35);
        assert_eq!(width(80, false), 20);
    }

    #[test]
    fn the_rendered_block_is_as_wide_as_it_says_in_every_combination() {
        let repos = [CLEAN, EDITS, AHEAD, BOTH, LARGE, HUGE];
        let batteries = [None, Some(discharging(5)), Some(discharging(100)), Some(plugged(42))];
        for client_width in TIERS {
            for repo in repos {
                for battery in batteries {
                    for key_table in ["root", "prefix", "pane"] {
                        let rendered = right(key_table, repo, battery).render(client_width);
                        assert_eq!(
                            cells(&rendered),
                            width(client_width, battery.is_some()),
                            "{rendered}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn the_pushed_block_leaves_the_key_slot_date_and_clock_to_tmux() {
        assert_eq!(
            pushed(BOTH, Some(discharging(87)), 120),
            "#[fg=#eebebe,nobold]+111 #[fg=#ea999c,nobold]−113 #[fg=#99d1db,nobold]↑2 #[fg=#99d1db,nobold]↓1    #[fg=#81c8be,nobold]\u{f0081} 87%%     #[fg=#949cbb,nobold]%a %d %b   #[fg=#c6d0f5,bold]"
        );
        assert_eq!(
            pushed(CLEAN, None, 80),
            "   #[fg=#c6d0f5,bold]"
        );
    }

    #[test]
    fn the_battery_icon_follows_the_charge_in_tenths() {
        let icons: String = [0, 19, 20, 35, 47, 50, 66, 79, 87, 95, 100]
            .into_iter()
            .map(|percent| discharging(percent).icon())
            .collect();
        assert_eq!(
            icons,
            "\u{f007a}\u{f007a}\u{f007b}\u{f007c}\u{f007d}\u{f007e}\u{f007f}\u{f0080}\u{f0081}\u{f0082}\u{f0079}"
        );
    }

    #[test]
    fn a_low_battery_on_the_charger_is_not_shown_as_a_warning() {
        assert_eq!(
            right("root", CLEAN, Some(plugged(12))).render(120),
            "#[fg=#8caaee,bold]                             #[fg=#81c8be,nobold]\u{f0084} 12%     #[fg=#949cbb,nobold]Tue 04 Aug   #[fg=#c6d0f5,bold]09:05#[nobold]  "
        );
    }
}
