use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::session::Checkout;

const BAR: &str = "#414559";
const CHIP: &str = "#51576d";
const CRUST: &str = "#303446";
const TEXT: &str = "#c6d0f5";
const MAUVE: &str = "#ca9ee6";
const PEACH: &str = "#ef9f76";
const DIM: &str = "#949cbb";

const LEFT_CAP: char = '\u{e0b6}';
const RIGHT_CAP: char = '\u{e0b4}';

const MAX_PROJECT: usize = 24;
const MIN_LIST: usize = 36;

pub struct Left<'a> {
    pub project: &'a str,
    pub checkout: Option<Checkout>,
}

impl Left<'_> {
    pub fn render(&self, client_width: u16, right_width: usize) -> String {
        let project = clip(self.project, MAX_PROJECT);
        let mut out = String::new();
        let mut cells = 0;
        match self.checkout {
            None => cells += capsule(&mut out, CHIP, TEXT, &project),
            Some(checkout) => {
                cells += capsule(&mut out, MAUVE, CRUST, &project);
                out.push_str(&format!("#[bg={BAR}] "));
                cells += 1;
                let (kind, fg) = match checkout {
                    Checkout::Root => ("root", DIM),
                    Checkout::Worktree => ("wt", PEACH),
                };
                cells += capsule(&mut out, CHIP, fg, kind);
            }
        }
        let room = usize::from(client_width).saturating_sub(right_width + MIN_LIST);
        let target = right_width.min(room).max(cells);
        out.push_str(&format!("#[bg={BAR},fg={DIM},nobold]"));
        out.push_str(&" ".repeat(target - cells));
        out
    }
}

fn capsule(out: &mut String, bg: &str, fg: &str, text: &str) -> usize {
    out.push_str(&format!(
        "#[fg={bg},bg={BAR}]{LEFT_CAP}#[bg={bg},fg={fg},bold] {text} #[fg={bg},bg={BAR},nobold]{RIGHT_CAP}"
    ));
    text.width() + 4
}

fn clip(text: &str, max: usize) -> String {
    if text.width() <= max {
        return text.to_string();
    }
    let mut out = String::new();
    let mut cells = 0;
    for c in text.chars() {
        let w = c.width().unwrap_or(0);
        if cells + w > max - 1 {
            break;
        }
        cells += w;
        out.push(c);
    }
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bar::right;

    const LONG: &str = "a-really-long-project-name-that-overflows";

    type BashRow = (&'static str, Option<Checkout>, u16, usize, &'static str);

    #[rustfmt::skip]
    const BASH: &[BashRow] = &[
        ("scratch", None, 60, 20, "#[fg=#51576d,bg=#414559]\u{e0b6}#[bg=#51576d,fg=#c6d0f5,bold] scratch #[fg=#51576d,bg=#414559,nobold]\u{e0b4}#[bg=#414559,fg=#949cbb,nobold]"),
        ("scratch", None, 80, 20, "#[fg=#51576d,bg=#414559]\u{e0b6}#[bg=#51576d,fg=#c6d0f5,bold] scratch #[fg=#51576d,bg=#414559,nobold]\u{e0b4}#[bg=#414559,fg=#949cbb,nobold]         "),
        ("scratch", None, 90, 35, "#[fg=#51576d,bg=#414559]\u{e0b6}#[bg=#51576d,fg=#c6d0f5,bold] scratch #[fg=#51576d,bg=#414559,nobold]\u{e0b4}#[bg=#414559,fg=#949cbb,nobold]        "),
        ("scratch", None, 99, 35, "#[fg=#51576d,bg=#414559]\u{e0b6}#[bg=#51576d,fg=#c6d0f5,bold] scratch #[fg=#51576d,bg=#414559,nobold]\u{e0b4}#[bg=#414559,fg=#949cbb,nobold]                 "),
        ("scratch", None, 100, 35, "#[fg=#51576d,bg=#414559]\u{e0b6}#[bg=#51576d,fg=#c6d0f5,bold] scratch #[fg=#51576d,bg=#414559,nobold]\u{e0b4}#[bg=#414559,fg=#949cbb,nobold]                  "),
        ("scratch", None, 119, 35, "#[fg=#51576d,bg=#414559]\u{e0b6}#[bg=#51576d,fg=#c6d0f5,bold] scratch #[fg=#51576d,bg=#414559,nobold]\u{e0b4}#[bg=#414559,fg=#949cbb,nobold]                        "),
        ("scratch", None, 120, 49, "#[fg=#51576d,bg=#414559]\u{e0b6}#[bg=#51576d,fg=#c6d0f5,bold] scratch #[fg=#51576d,bg=#414559,nobold]\u{e0b4}#[bg=#414559,fg=#949cbb,nobold]                        "),
        ("scratch", None, 200, 49, "#[fg=#51576d,bg=#414559]\u{e0b6}#[bg=#51576d,fg=#c6d0f5,bold] scratch #[fg=#51576d,bg=#414559,nobold]\u{e0b4}#[bg=#414559,fg=#949cbb,nobold]                                      "),
        ("bp-api", Some(Checkout::Root), 60, 20, "#[fg=#ca9ee6,bg=#414559]\u{e0b6}#[bg=#ca9ee6,fg=#303446,bold] bp-api #[fg=#ca9ee6,bg=#414559,nobold]\u{e0b4}#[bg=#414559] #[fg=#51576d,bg=#414559]\u{e0b6}#[bg=#51576d,fg=#949cbb,bold] root #[fg=#51576d,bg=#414559,nobold]\u{e0b4}#[bg=#414559,fg=#949cbb,nobold]"),
        ("bp-api", Some(Checkout::Root), 80, 20, "#[fg=#ca9ee6,bg=#414559]\u{e0b6}#[bg=#ca9ee6,fg=#303446,bold] bp-api #[fg=#ca9ee6,bg=#414559,nobold]\u{e0b4}#[bg=#414559] #[fg=#51576d,bg=#414559]\u{e0b6}#[bg=#51576d,fg=#949cbb,bold] root #[fg=#51576d,bg=#414559,nobold]\u{e0b4}#[bg=#414559,fg=#949cbb,nobold] "),
        ("bp-api", Some(Checkout::Root), 90, 35, "#[fg=#ca9ee6,bg=#414559]\u{e0b6}#[bg=#ca9ee6,fg=#303446,bold] bp-api #[fg=#ca9ee6,bg=#414559,nobold]\u{e0b4}#[bg=#414559] #[fg=#51576d,bg=#414559]\u{e0b6}#[bg=#51576d,fg=#949cbb,bold] root #[fg=#51576d,bg=#414559,nobold]\u{e0b4}#[bg=#414559,fg=#949cbb,nobold]"),
        ("bp-api", Some(Checkout::Root), 99, 35, "#[fg=#ca9ee6,bg=#414559]\u{e0b6}#[bg=#ca9ee6,fg=#303446,bold] bp-api #[fg=#ca9ee6,bg=#414559,nobold]\u{e0b4}#[bg=#414559] #[fg=#51576d,bg=#414559]\u{e0b6}#[bg=#51576d,fg=#949cbb,bold] root #[fg=#51576d,bg=#414559,nobold]\u{e0b4}#[bg=#414559,fg=#949cbb,nobold]         "),
        ("bp-api", Some(Checkout::Root), 100, 35, "#[fg=#ca9ee6,bg=#414559]\u{e0b6}#[bg=#ca9ee6,fg=#303446,bold] bp-api #[fg=#ca9ee6,bg=#414559,nobold]\u{e0b4}#[bg=#414559] #[fg=#51576d,bg=#414559]\u{e0b6}#[bg=#51576d,fg=#949cbb,bold] root #[fg=#51576d,bg=#414559,nobold]\u{e0b4}#[bg=#414559,fg=#949cbb,nobold]          "),
        ("bp-api", Some(Checkout::Root), 119, 35, "#[fg=#ca9ee6,bg=#414559]\u{e0b6}#[bg=#ca9ee6,fg=#303446,bold] bp-api #[fg=#ca9ee6,bg=#414559,nobold]\u{e0b4}#[bg=#414559] #[fg=#51576d,bg=#414559]\u{e0b6}#[bg=#51576d,fg=#949cbb,bold] root #[fg=#51576d,bg=#414559,nobold]\u{e0b4}#[bg=#414559,fg=#949cbb,nobold]                "),
        ("bp-api", Some(Checkout::Root), 120, 49, "#[fg=#ca9ee6,bg=#414559]\u{e0b6}#[bg=#ca9ee6,fg=#303446,bold] bp-api #[fg=#ca9ee6,bg=#414559,nobold]\u{e0b4}#[bg=#414559] #[fg=#51576d,bg=#414559]\u{e0b6}#[bg=#51576d,fg=#949cbb,bold] root #[fg=#51576d,bg=#414559,nobold]\u{e0b4}#[bg=#414559,fg=#949cbb,nobold]                "),
        ("bp-api", Some(Checkout::Root), 200, 49, "#[fg=#ca9ee6,bg=#414559]\u{e0b6}#[bg=#ca9ee6,fg=#303446,bold] bp-api #[fg=#ca9ee6,bg=#414559,nobold]\u{e0b4}#[bg=#414559] #[fg=#51576d,bg=#414559]\u{e0b6}#[bg=#51576d,fg=#949cbb,bold] root #[fg=#51576d,bg=#414559,nobold]\u{e0b4}#[bg=#414559,fg=#949cbb,nobold]                              "),
        ("bp-api", Some(Checkout::Worktree), 60, 20, "#[fg=#ca9ee6,bg=#414559]\u{e0b6}#[bg=#ca9ee6,fg=#303446,bold] bp-api #[fg=#ca9ee6,bg=#414559,nobold]\u{e0b4}#[bg=#414559] #[fg=#51576d,bg=#414559]\u{e0b6}#[bg=#51576d,fg=#ef9f76,bold] wt #[fg=#51576d,bg=#414559,nobold]\u{e0b4}#[bg=#414559,fg=#949cbb,nobold]"),
        ("bp-api", Some(Checkout::Worktree), 80, 20, "#[fg=#ca9ee6,bg=#414559]\u{e0b6}#[bg=#ca9ee6,fg=#303446,bold] bp-api #[fg=#ca9ee6,bg=#414559,nobold]\u{e0b4}#[bg=#414559] #[fg=#51576d,bg=#414559]\u{e0b6}#[bg=#51576d,fg=#ef9f76,bold] wt #[fg=#51576d,bg=#414559,nobold]\u{e0b4}#[bg=#414559,fg=#949cbb,nobold]   "),
        ("bp-api", Some(Checkout::Worktree), 90, 35, "#[fg=#ca9ee6,bg=#414559]\u{e0b6}#[bg=#ca9ee6,fg=#303446,bold] bp-api #[fg=#ca9ee6,bg=#414559,nobold]\u{e0b4}#[bg=#414559] #[fg=#51576d,bg=#414559]\u{e0b6}#[bg=#51576d,fg=#ef9f76,bold] wt #[fg=#51576d,bg=#414559,nobold]\u{e0b4}#[bg=#414559,fg=#949cbb,nobold]  "),
        ("bp-api", Some(Checkout::Worktree), 99, 35, "#[fg=#ca9ee6,bg=#414559]\u{e0b6}#[bg=#ca9ee6,fg=#303446,bold] bp-api #[fg=#ca9ee6,bg=#414559,nobold]\u{e0b4}#[bg=#414559] #[fg=#51576d,bg=#414559]\u{e0b6}#[bg=#51576d,fg=#ef9f76,bold] wt #[fg=#51576d,bg=#414559,nobold]\u{e0b4}#[bg=#414559,fg=#949cbb,nobold]           "),
        ("bp-api", Some(Checkout::Worktree), 100, 35, "#[fg=#ca9ee6,bg=#414559]\u{e0b6}#[bg=#ca9ee6,fg=#303446,bold] bp-api #[fg=#ca9ee6,bg=#414559,nobold]\u{e0b4}#[bg=#414559] #[fg=#51576d,bg=#414559]\u{e0b6}#[bg=#51576d,fg=#ef9f76,bold] wt #[fg=#51576d,bg=#414559,nobold]\u{e0b4}#[bg=#414559,fg=#949cbb,nobold]            "),
        ("bp-api", Some(Checkout::Worktree), 119, 35, "#[fg=#ca9ee6,bg=#414559]\u{e0b6}#[bg=#ca9ee6,fg=#303446,bold] bp-api #[fg=#ca9ee6,bg=#414559,nobold]\u{e0b4}#[bg=#414559] #[fg=#51576d,bg=#414559]\u{e0b6}#[bg=#51576d,fg=#ef9f76,bold] wt #[fg=#51576d,bg=#414559,nobold]\u{e0b4}#[bg=#414559,fg=#949cbb,nobold]                  "),
        ("bp-api", Some(Checkout::Worktree), 120, 49, "#[fg=#ca9ee6,bg=#414559]\u{e0b6}#[bg=#ca9ee6,fg=#303446,bold] bp-api #[fg=#ca9ee6,bg=#414559,nobold]\u{e0b4}#[bg=#414559] #[fg=#51576d,bg=#414559]\u{e0b6}#[bg=#51576d,fg=#ef9f76,bold] wt #[fg=#51576d,bg=#414559,nobold]\u{e0b4}#[bg=#414559,fg=#949cbb,nobold]                  "),
        ("bp-api", Some(Checkout::Worktree), 200, 49, "#[fg=#ca9ee6,bg=#414559]\u{e0b6}#[bg=#ca9ee6,fg=#303446,bold] bp-api #[fg=#ca9ee6,bg=#414559,nobold]\u{e0b4}#[bg=#414559] #[fg=#51576d,bg=#414559]\u{e0b6}#[bg=#51576d,fg=#ef9f76,bold] wt #[fg=#51576d,bg=#414559,nobold]\u{e0b4}#[bg=#414559,fg=#949cbb,nobold]                                "),
        (LONG, Some(Checkout::Root), 60, 20, "#[fg=#ca9ee6,bg=#414559]\u{e0b6}#[bg=#ca9ee6,fg=#303446,bold] a-really-long-project-n… #[fg=#ca9ee6,bg=#414559,nobold]\u{e0b4}#[bg=#414559] #[fg=#51576d,bg=#414559]\u{e0b6}#[bg=#51576d,fg=#949cbb,bold] root #[fg=#51576d,bg=#414559,nobold]\u{e0b4}#[bg=#414559,fg=#949cbb,nobold]"),
        (LONG, Some(Checkout::Root), 80, 20, "#[fg=#ca9ee6,bg=#414559]\u{e0b6}#[bg=#ca9ee6,fg=#303446,bold] a-really-long-project-n… #[fg=#ca9ee6,bg=#414559,nobold]\u{e0b4}#[bg=#414559] #[fg=#51576d,bg=#414559]\u{e0b6}#[bg=#51576d,fg=#949cbb,bold] root #[fg=#51576d,bg=#414559,nobold]\u{e0b4}#[bg=#414559,fg=#949cbb,nobold]"),
        (LONG, Some(Checkout::Root), 90, 35, "#[fg=#ca9ee6,bg=#414559]\u{e0b6}#[bg=#ca9ee6,fg=#303446,bold] a-really-long-project-n… #[fg=#ca9ee6,bg=#414559,nobold]\u{e0b4}#[bg=#414559] #[fg=#51576d,bg=#414559]\u{e0b6}#[bg=#51576d,fg=#949cbb,bold] root #[fg=#51576d,bg=#414559,nobold]\u{e0b4}#[bg=#414559,fg=#949cbb,nobold]"),
        (LONG, Some(Checkout::Root), 99, 35, "#[fg=#ca9ee6,bg=#414559]\u{e0b6}#[bg=#ca9ee6,fg=#303446,bold] a-really-long-project-n… #[fg=#ca9ee6,bg=#414559,nobold]\u{e0b4}#[bg=#414559] #[fg=#51576d,bg=#414559]\u{e0b6}#[bg=#51576d,fg=#949cbb,bold] root #[fg=#51576d,bg=#414559,nobold]\u{e0b4}#[bg=#414559,fg=#949cbb,nobold]"),
        (LONG, Some(Checkout::Root), 100, 35, "#[fg=#ca9ee6,bg=#414559]\u{e0b6}#[bg=#ca9ee6,fg=#303446,bold] a-really-long-project-n… #[fg=#ca9ee6,bg=#414559,nobold]\u{e0b4}#[bg=#414559] #[fg=#51576d,bg=#414559]\u{e0b6}#[bg=#51576d,fg=#949cbb,bold] root #[fg=#51576d,bg=#414559,nobold]\u{e0b4}#[bg=#414559,fg=#949cbb,nobold]"),
        (LONG, Some(Checkout::Root), 119, 35, "#[fg=#ca9ee6,bg=#414559]\u{e0b6}#[bg=#ca9ee6,fg=#303446,bold] a-really-long-project-n… #[fg=#ca9ee6,bg=#414559,nobold]\u{e0b4}#[bg=#414559] #[fg=#51576d,bg=#414559]\u{e0b6}#[bg=#51576d,fg=#949cbb,bold] root #[fg=#51576d,bg=#414559,nobold]\u{e0b4}#[bg=#414559,fg=#949cbb,nobold]"),
        (LONG, Some(Checkout::Root), 120, 49, "#[fg=#ca9ee6,bg=#414559]\u{e0b6}#[bg=#ca9ee6,fg=#303446,bold] a-really-long-project-n… #[fg=#ca9ee6,bg=#414559,nobold]\u{e0b4}#[bg=#414559] #[fg=#51576d,bg=#414559]\u{e0b6}#[bg=#51576d,fg=#949cbb,bold] root #[fg=#51576d,bg=#414559,nobold]\u{e0b4}#[bg=#414559,fg=#949cbb,nobold]"),
        (LONG, Some(Checkout::Root), 200, 49, "#[fg=#ca9ee6,bg=#414559]\u{e0b6}#[bg=#ca9ee6,fg=#303446,bold] a-really-long-project-n… #[fg=#ca9ee6,bg=#414559,nobold]\u{e0b4}#[bg=#414559] #[fg=#51576d,bg=#414559]\u{e0b6}#[bg=#51576d,fg=#949cbb,bold] root #[fg=#51576d,bg=#414559,nobold]\u{e0b4}#[bg=#414559,fg=#949cbb,nobold]            "),
    ];

    #[test]
    fn renders_exactly_what_the_bash_script_it_replaced_rendered() {
        for &(project, checkout, client_width, right_width, bash) in BASH {
            let left = Left { project, checkout };
            assert_eq!(
                left.render(client_width, right_width),
                bash,
                "{project} at {client_width} columns"
            );
        }
    }

    const TIERS: [u16; 7] = [60, 80, 90, 100, 120, 160, 200];

    fn cells(rendered: &str) -> usize {
        let mut visible = String::new();
        let mut rest = rendered;
        while let Some(start) = rest.find("#[") {
            visible.push_str(&rest[..start]);
            rest = &rest[start..];
            rest = &rest[rest.find(']').unwrap() + 1..];
        }
        visible.push_str(rest);
        visible.width()
    }

    fn snapshot_tiers(name: &str, left: &Left) {
        let rendered: Vec<String> = TIERS
            .iter()
            .flat_map(|&client_width| {
                [true, false].map(|battery| {
                    let right_width = right::width(client_width, battery);
                    let block = left.render(client_width, right_width);
                    format!(
                        "{client_width:>3} right {right_width:>2} left {:>2} |{block}|",
                        cells(&block)
                    )
                })
            })
            .collect();
        insta::assert_snapshot!(name, rendered.join("\n"));
    }

    #[test]
    fn every_tier_at_the_main_checkout() {
        snapshot_tiers(
            "short_root",
            &Left {
                project: "bp-api",
                checkout: Some(Checkout::Root),
            },
        );
    }

    #[test]
    fn every_tier_in_a_linked_worktree() {
        snapshot_tiers(
            "short_worktree",
            &Left {
                project: "bp-api",
                checkout: Some(Checkout::Worktree),
            },
        );
    }

    #[test]
    fn every_tier_with_a_long_project_name() {
        snapshot_tiers(
            "long_worktree",
            &Left {
                project: LONG,
                checkout: Some(Checkout::Worktree),
            },
        );
    }

    #[test]
    fn every_tier_outside_any_repo() {
        snapshot_tiers(
            "outside_repo",
            &Left {
                project: "scratch",
                checkout: None,
            },
        );
    }

    #[test]
    fn the_padding_matches_the_right_block_wherever_the_list_fits_centred() {
        for battery in [true, false] {
            for client_width in [200, 160] {
                let right_width = right::width(client_width, battery);
                for checkout in [None, Some(Checkout::Root), Some(Checkout::Worktree)] {
                    for project in ["x", "bp-api", LONG] {
                        let block = Left { project, checkout }.render(client_width, right_width);
                        assert_eq!(cells(&block), right_width, "{block}");
                    }
                }
            }
        }
    }

    #[test]
    fn below_the_widest_tier_the_padding_gives_way_to_the_list() {
        let left = Left {
            project: "bp-api",
            checkout: Some(Checkout::Root),
        };
        assert_eq!(cells(&left.render(120, 59)), 25);
        assert_eq!(cells(&left.render(100, 45)), 19);
        assert_eq!(cells(&left.render(80, 20)), 20);
        assert_eq!(cells(&left.render(60, 20)), 19);
    }

    #[test]
    fn a_long_name_is_clipped_to_the_cap_with_an_ellipsis() {
        let block = Left {
            project: LONG,
            checkout: Some(Checkout::Root),
        }
        .render(200, 59);
        assert!(block.contains(" a-really-long-project-n… "), "{block}");
        assert_eq!(cells(&block), 59);
    }

    #[test]
    fn a_name_of_exactly_the_cap_is_left_whole() {
        let block = Left {
            project: "abcdefghijklmnopqrstuvwx",
            checkout: None,
        }
        .render(200, 59);
        assert!(block.contains(" abcdefghijklmnopqrstuvwx "), "{block}");
    }

    #[test]
    fn wide_characters_are_clipped_by_cells_not_characters() {
        let block = Left {
            project: "日本語のプロジェクトの名前がとても長い",
            checkout: Some(Checkout::Root),
        }
        .render(200, 59);
        assert!(block.contains(" 日本語のプロジェクトの… "), "{block}");
        assert_eq!(cells(&block), 59);
    }
}
