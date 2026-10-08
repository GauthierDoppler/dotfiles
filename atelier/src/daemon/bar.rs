use std::collections::HashMap;

use super::repos::Repos;
use super::state::Watched;
use crate::bar::{self, Battery, Pushed};
use crate::session::Field;
use crate::shell::quote;

pub const RESIZED: &str = "atelier:client-resized";

pub fn adopt(me: &str) -> Vec<String> {
    vec![
        format!("set-option -g @bar_daemon {}", quote(me)),
        format!(
            "set-hook -g client-resized[73] {}",
            quote(&format!("display-message -c {me} {RESIZED}"))
        ),
        "set-hook -g client-detached[73] \"if -F '#{==:#{hook_client},#{@bar_daemon}}' 'set-option -gu @bar_daemon ; set-hook -gu client-resized[73]'\"".to_string(),
    ]
}

#[derive(Default)]
pub struct Pusher {
    shown: HashMap<String, Pushed>,
    battery: Option<Battery>,
}

impl Pusher {
    pub fn battery_changed(&mut self) -> bool {
        let battery = Battery::read();
        let changed = battery != self.battery;
        self.battery = battery;
        changed
    }

    pub fn commands(&mut self, watched: Vec<Watched>, repos: &mut Repos) -> Vec<String> {
        self.battery = Battery::read();
        let mut commands = Vec::new();
        for session in watched {
            let Some(identity) = repos.identity(&session.fields) else {
                continue;
            };
            let counts = repos.counts(&session.fields[Field::Path as usize]);
            let blocks = bar::pushed(&identity, counts, session.width, self.battery);
            if self.shown.get(&session.session) == Some(&blocks) {
                continue;
            }
            let target = quote(&session.session);
            commands.push(format!(
                "set-option -t {target} @bar_left {}",
                quote(&blocks.left)
            ));
            commands.push(format!(
                "set-option -t {target} @bar_right {}",
                quote(&blocks.right)
            ));
            self.shown.insert(session.session, blocks);
        }
        commands
    }
}
