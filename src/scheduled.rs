use std::sync::Arc;

use clokwerk::{Scheduler, TimeUnits};
use tokio::runtime::Handle;

use crate::{
    database::Database,
    utils::{giveaway_updater::update_giveaways, winner_showcase::display_winner},
    Http,
};

type SetupFn = fn(&mut Scheduler, Handle, Arc<Http>, Arc<Database>) -> ();

pub const SETUP_FUNCTIONS: [&SetupFn; 2] = [
    &(setup_display_winner as SetupFn),
    &(setup_update_giveaways as SetupFn),
];

fn setup_display_winner(
    scheduler: &mut Scheduler,
    handle: Handle,
    http: Arc<Http>,
    db: Arc<Database>,
) {
    scheduler.every(1.day()).at("00:00").run(move || {
        handle.block_on(display_winner(http.clone(), db.clone(), 1));
    });
}

fn setup_update_giveaways(
    scheduler: &mut Scheduler,
    handle: Handle,
    http: Arc<Http>,
    db: Arc<Database>,
) {
    scheduler.every(30.seconds()).run(move || {
        handle.block_on(update_giveaways(http.clone(), db.clone()));
    });
}
