#[derive(Debug, PartialEq)]
pub enum Event {
    Reply(Reply),
    Notification(Notification),
}

#[derive(Debug, PartialEq)]
pub struct Reply {
    pub ours: bool,
    pub ok: bool,
    pub lines: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub enum Notification {
    SessionsChanged,
    SessionChanged,
    SessionRenamed { session: String, name: String },
    WindowAdd,
    WindowClose { window: String },
    WindowRenamed { window: String, name: String },
    ClientSessionChanged,
    ClientDetached,
    Message(String),
    Exit,
    Ignored,
}

struct Open {
    number: String,
    reply: Reply,
}

#[derive(Default)]
pub struct Parser {
    open: Option<Open>,
}

impl Parser {
    pub fn feed(&mut self, line: &str) -> Option<Event> {
        if let Some(open) = &mut self.open {
            match guard(line) {
                Some((kind @ ("%end" | "%error"), number, _)) if number == open.number => {
                    let mut reply = self.open.take()?.reply;
                    reply.ok = kind == "%end";
                    return Some(Event::Reply(reply));
                }
                _ => {
                    open.reply.lines.push(line.to_string());
                    return None;
                }
            }
        }
        if let Some(("%begin", number, flags)) = guard(line) {
            self.open = Some(Open {
                number: number.to_string(),
                reply: Reply {
                    ours: flags.parse::<u32>().is_ok_and(|f| f & 1 == 1),
                    ok: false,
                    lines: Vec::new(),
                },
            });
            return None;
        }
        Some(Event::Notification(notification(line)))
    }
}

fn guard(line: &str) -> Option<(&str, &str, &str)> {
    let mut words = line.split(' ');
    let kind = words.next()?;
    let (_time, number, flags) = (words.next()?, words.next()?, words.next()?);
    words.next().is_none().then_some((kind, number, flags))
}

fn notification(line: &str) -> Notification {
    let (kind, rest) = line.split_once(' ').unwrap_or((line, ""));
    let (id, name) = rest.split_once(' ').unwrap_or((rest, ""));
    match kind {
        "%sessions-changed" => Notification::SessionsChanged,
        "%session-changed" => Notification::SessionChanged,
        "%session-renamed" => Notification::SessionRenamed {
            session: id.to_string(),
            name: name.to_string(),
        },
        "%window-add" | "%unlinked-window-add" => Notification::WindowAdd,
        "%window-close" | "%unlinked-window-close" => Notification::WindowClose {
            window: id.to_string(),
        },
        "%window-renamed" | "%unlinked-window-renamed" => Notification::WindowRenamed {
            window: id.to_string(),
            name: name.to_string(),
        },
        "%client-session-changed" => Notification::ClientSessionChanged,
        "%client-detached" => Notification::ClientDetached,
        "%message" => Notification::Message(rest.to_string()),
        "%exit" => Notification::Exit,
        _ => Notification::Ignored,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(transcript: &str) -> Vec<Event> {
        let mut parser = Parser::default();
        transcript.lines().filter_map(|l| parser.feed(l)).collect()
    }

    fn reply(ours: bool, ok: bool, lines: &[&str]) -> Event {
        Event::Reply(Reply {
            ours,
            ok,
            lines: lines.iter().map(|l| l.to_string()).collect(),
        })
    }

    fn note(notification: Notification) -> Event {
        Event::Notification(notification)
    }

    fn renamed_window(window: &str, name: &str) -> Event {
        note(Notification::WindowRenamed {
            window: window.into(),
            name: name.into(),
        })
    }

    fn closed_window(window: &str) -> Event {
        note(Notification::WindowClose {
            window: window.into(),
        })
    }

    #[test]
    fn a_recorded_session_lifecycle_parses_into_replies_and_notifications() {
        use Notification::*;
        assert_eq!(
            parse(include_str!("../../tests/fixtures/control/lifecycle.txt")),
            vec![
                reply(false, true, &[]),
                note(SessionChanged),
                renamed_window("@0", "sh"),
                renamed_window("@0", "%end 1 1 1"),
                reply(true, true, &["$0 work"]),
                reply(true, true, &["$0 @0 0 %end 1 1 1"]),
                reply(true, false, &["can't find session: nosuch"]),
                note(WindowAdd),
                note(SessionsChanged),
                note(ClientSessionChanged),
                note(ClientSessionChanged),
                note(ClientDetached),
                note(Ignored),
                note(WindowAdd),
                note(Ignored),
                note(WindowAdd),
                renamed_window("@2", "tail"),
                renamed_window("@3", "make"),
                note(SessionRenamed {
                    session: "$1".into(),
                    name: "backend".into()
                }),
                note(Ignored),
                closed_window("@2"),
                note(SessionsChanged),
                closed_window("@1"),
                closed_window("@3"),
                note(WindowAdd),
                note(SessionsChanged),
                note(SessionChanged),
                note(SessionsChanged),
                note(SessionsChanged),
                note(Exit),
            ]
        );
    }

    #[test]
    fn pane_output_is_ignored_even_when_it_looks_like_a_block_end() {
        use Notification::*;
        assert_eq!(
            parse(include_str!("../../tests/fixtures/control/output.txt")),
            vec![
                reply(false, true, &[]),
                note(SessionChanged),
                renamed_window("@0", "sh"),
                note(Ignored),
                note(SessionsChanged),
                note(Exit),
            ]
        );
    }

    #[test]
    fn a_killed_session_detaches_the_control_client_with_exit() {
        use Notification::*;
        assert_eq!(
            parse(include_str!("../../tests/fixtures/control/session-killed.txt")),
            vec![
                reply(false, true, &[]),
                note(SessionChanged),
                renamed_window("@0", "sh"),
                note(WindowAdd),
                note(SessionsChanged),
                note(SessionsChanged),
                note(Exit),
            ]
        );
    }

    #[test]
    fn a_resize_reaches_the_control_client_as_the_message_its_hook_displays() {
        use Notification::*;
        let resized = || note(Message("atelier:client-resized".into()));
        assert_eq!(
            parse(include_str!("../../tests/fixtures/control/client-resized.txt")),
            vec![
                reply(false, true, &[]),
                note(SessionChanged),
                renamed_window("@0", "sh"),
                reply(true, true, &[]),
                note(WindowAdd),
                note(SessionsChanged),
                note(ClientSessionChanged),
                note(Ignored),
                resized(),
                renamed_window("@1", "tmux"),
                resized(),
                note(Ignored),
                note(SessionsChanged),
                note(SessionsChanged),
                note(ClientDetached),
                note(Exit),
            ]
        );
    }

    #[test]
    fn a_block_line_that_mimics_another_command_number_stays_output() {
        assert_eq!(
            parse("%begin 1 7 1\n%end 1 6 1\n%begin 1 8 1\n%end 1 7 1\n"),
            vec![reply(true, true, &["%end 1 6 1", "%begin 1 8 1"])]
        );
    }
}
