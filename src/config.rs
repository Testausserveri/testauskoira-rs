use clap::Parser;
use std::sync::LazyLock;

pub static CONFIG: LazyLock<Config> = LazyLock::new(Config::parse);

/// Bot configuration, parsed from environment variables at startup.
#[derive(Parser)]
#[command(about = "Testausserveri Discord bot")]
pub struct Config {
    /// MySQL/MariaDB connection string (e.g. mysql://root@localhost/testauskoira)
    #[arg(long, env)]
    pub database_url: String,

    /// Discord bot token
    #[arg(long, env)]
    pub discord_token: String,

    /// Discord server (guild) ID the bot operates in
    #[arg(long, env)]
    pub guild_id: u64,

    /// Channel where moderation/voting messages are posted
    #[arg(long, env)]
    pub mod_channel_id: u64,

    /// Role applied to silenced users
    #[arg(long, env)]
    pub silenced_role_id: u64,

    /// Role granted when a member passes membership screening
    #[arg(long, env)]
    pub member_role_id: u64,

    /// Role that prevents a user from submitting reports
    #[arg(long, env)]
    pub no_reports_role_id: u64,

    /// Role given to the weekly activity award winner
    #[arg(long, env)]
    pub award_role_id: u64,

    /// Channel where activity award announcements are posted
    #[arg(long, env)]
    pub award_channel_id: u64,

    /// Channel linked in the silence DM so users can review the rules
    #[arg(long, env)]
    pub rules_channel_id: u64,

    /// Channel where the bot posts a startup message (optional)
    #[arg(long, env, default_value = None)]
    pub status_channel_id: Option<u64>,

    /// Emoji used for giveaway reactions
    #[arg(long, env, default_value = "🎉")]
    pub giveaway_reaction_emoji: char,

    /// Default giveaway duration in seconds
    #[arg(long, env, default_value = "3600")]
    pub giveaway_default_duration: i64,

    /// Default number of giveaway winners
    #[arg(long, env, default_value = "1")]
    pub giveaway_default_winners: i64,

    /// Default giveaway prize description
    #[arg(long, env, default_value = "Nothing")]
    pub giveaway_default_prize: String,
}
