use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use tokio::sync::mpsc::UnboundedSender;

use super::bar::Pusher;
use super::repos::{Fields, Repos};
use super::state::Watched;

const BATTERY: Duration = Duration::from_secs(60);

pub enum Input {
    Push {
        watched: Vec<Watched>,
        sessions: Vec<Fields>,
    },
    Change(notify::Result<notify::Event>),
}

pub fn spawn(commands: UnboundedSender<Vec<String>>) -> Sender<Input> {
    let (input, inputs) = channel();
    let changes = input.clone();
    std::thread::spawn(move || {
        let repos = Repos::new(move |event| {
            let _ = changes.send(Input::Change(event));
        });
        Renderer {
            repos,
            pusher: Pusher::default(),
            watched: Vec::new(),
            battery_due: Instant::now() + BATTERY,
        }
        .run(&inputs, &commands);
    });
    input
}

struct Renderer {
    repos: Repos,
    pusher: Pusher,
    watched: Vec<Watched>,
    battery_due: Instant,
}

impl Renderer {
    fn run(mut self, inputs: &Receiver<Input>, commands: &UnboundedSender<Vec<String>>) {
        while !commands.is_closed() {
            let due = self
                .repos
                .due()
                .map_or(self.battery_due, |due| due.min(self.battery_due));
            let mut push = match inputs.recv_timeout(due.saturating_duration_since(Instant::now())) {
                Ok(input) => self.take(input),
                Err(RecvTimeoutError::Timeout) => self.tick(),
                Err(RecvTimeoutError::Disconnected) => return,
            };
            while let Ok(input) = inputs.try_recv() {
                push |= self.take(input);
            }
            if push {
                let pushed = self.pusher.commands(&self.watched, &mut self.repos);
                if !pushed.is_empty() && commands.send(pushed).is_err() {
                    return;
                }
            }
        }
    }

    fn take(&mut self, input: Input) -> bool {
        match input {
            Input::Push { watched, sessions } => {
                self.repos.follow(&sessions);
                self.watched = watched;
                true
            }
            Input::Change(change) => {
                self.repos.note(change);
                false
            }
        }
    }

    fn tick(&mut self) -> bool {
        let now = Instant::now();
        let mut push = self.repos.due().is_some_and(|due| due <= now) && self.repos.settle();
        if self.battery_due <= now {
            self.battery_due = now + BATTERY;
            push |= self.pusher.battery_changed();
        }
        push
    }
}
