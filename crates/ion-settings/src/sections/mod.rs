//! Every Settings section, one file each. The order of `all` is the order of
//! the sidebar; add a line here for a new section.

use crate::{Context, Section};

mod about;
mod agent;
mod appearance;
mod blocking;
mod downloads;
mod general;
mod gestures;
mod keyboard;
mod media;
mod new_tab;
mod passwords;
mod privacy;
mod search;

pub fn all(ctx: &Context) -> Vec<Section> {
    vec![
        general::section(ctx),
        appearance::section(ctx),
        search::section(ctx),
        new_tab::section(ctx),
        privacy::section(ctx),
        passwords::section(ctx),
        blocking::section(ctx),
        media::section(ctx),
        downloads::section(ctx),
        keyboard::section(ctx),
        gestures::section(ctx),
        agent::section(ctx),
        about::section(ctx),
    ]
}
