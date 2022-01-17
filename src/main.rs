pub mod commands;
pub mod database;
pub mod extensions;
pub mod models;
pub mod scheduled;
pub mod schema;
pub mod utils;
pub mod voting;

#[macro_use]
extern crate tracing;
#[macro_use]
extern crate diesel;

use std::{collections::HashSet, env, sync::Arc};

use clokwerk::Scheduler;
use commands::owner::*;
use database::Database;
use extensions::*;
use serenity::{
    async_trait,
    client::bridge::gateway::{GatewayIntents, ShardManager},
    framework::{standard::macros::group, StandardFramework},
    http::Http,
    model::{
        event::{MessageUpdateEvent, ResumedEvent},
        gateway::Ready,
        interactions::application_command::*,
        prelude::*,
    },
    prelude::*,
};

pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

pub struct BlacklistRegexes {
    last_edited: std::time::SystemTime,
    regexvec: Vec<regex::Regex>,
}

impl TypeMapKey for BlacklistRegexes {
    type Value = Arc<Mutex<BlacklistRegexes>>;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("Connected as {}", ready.user.name);
        let guild_id: u64 = env::var("GUILD_ID")
            .expect("No GUILD_ID in .env")
            .parse()
            .expect("Invalid GUILD_ID provided");
        let guild_id = serenity::model::id::GuildId::from(guild_id);
        let admin_role_id: u64 = env::var("ADMIN_ROLE_ID")
            .expect("No ADMIN_ROLE_ID in .env")
            .parse()
            .expect("Invalid ADMIN_ROLE_ID provided");
        let commands = guild_id
            .set_application_commands(&ctx.http, |commands| {
                commands.create_application_command(|command| {
                    command
                        .name("github")
                        .description("Vastaanota kutsu Testausserverin GitHub-organisaatioon")
                });
                commands.create_application_command(|command| {
                    command
                        .name("giveaway")
                        .description("Luo arpajaistapahtuma tai hallinnoi olemassaolevia arpajaistapahtumia (:D)")
                        .default_permission(false)
                        .create_option(|option| {
                            option
                                .name("start")
                                .description("Luo ja aloita arpajaistapahtuma")
                                .kind(ApplicationCommandOptionType::SubCommand)
                                .create_sub_option(|subopt| {
                                    subopt
                                        .name("channel")
                                        .description("Arpajaisilmoituksen kanava")
                                        .required(true)
                                        .channel_types(&[serenity::model::channel::ChannelType::Text])
                                        .kind(ApplicationCommandOptionType::Channel)
                                })
                                .create_sub_option(|subopt| {
                                    subopt
                                        .name("duration")
                                        .description("Arpajaistapahtuman kesto (sekunneissa)")
                                        .kind(ApplicationCommandOptionType::Integer)
                                })
                                .create_sub_option(|subopt| {
                                    subopt
                                        .name("winners")
                                        .description("Arpajaistapahtuman voittajien lukumäärä")
                                        .kind(ApplicationCommandOptionType::Integer)
                                })
                                .create_sub_option(|subopt| {
                                    subopt
                                        .name("prize")
                                        .description("Arpajaistapahtuman palkinto")
                                        .kind(ApplicationCommandOptionType::String)
                                })
                                .create_sub_option(|subopt| {
                                    subopt
                                        .name("mention")
                                        .description("Rooli joka mainitaan arpajaistapahtuman aloitettaessa")
                                        .kind(ApplicationCommandOptionType::Role)
                                })
                        })
                        .create_option(|option| {
                            option
                                .name("list")
                                .description("Luetteloi arpajaistapahtumat")
                                .kind(ApplicationCommandOptionType::SubCommand)
                        })
                        .create_option(|option| {
                            option
                                .name("reroll")
                                .kind(ApplicationCommandOptionType::SubCommand)
                                .description("Valitse uudelleen arpajaistapahtuman voittaja(t)")
                                .create_sub_option(|subopt| {
                                    subopt
                                        .name("giveaway_id")
                                        .required(true)
                                        .description("Arpajaistapahtuman id")
                                        .kind(ApplicationCommandOptionType::Integer)
                                })
                        })
                        .create_option(|option| {
                            option
                                .name("edit")
                                .kind(ApplicationCommandOptionType::SubCommand)
                                .description("Muokkaa arpajaistapahtumaa")
                                .create_sub_option(|subopt| {
                                    subopt
                                        .name("giveaway_id")
                                        .required(true)
                                        .description("Arpajaistapahtuman id")
                                        .kind(ApplicationCommandOptionType::Integer)
                                })
                                .create_sub_option(|subopt| {
                                    subopt
                                        .name("field")
                                        .required(true)
                                        .description("Muokattava atribuutti/kenttä")
                                        .kind(ApplicationCommandOptionType::String)
                                        .add_string_choice("Arpajaistapahtuman kesto", "duration")
                                        .add_string_choice("Arpajaistapahtuman voittajien lukumäärä", "winners")
                                })
                                .create_sub_option(|subopt| {
                                    subopt
                                        .name("new_value")
                                        .required(true)
                                        .description("Uusi arvo")
                                        .kind(ApplicationCommandOptionType::Integer)
                                })
                        })
                        .create_option(|option| {
                            option
                                .name("end")
                                .kind(ApplicationCommandOptionType::SubCommand)
                                .description("Lopeta arpajaistapahtuma")
                                .create_sub_option(|subopt| {
                                    subopt
                                        .name("giveaway_id")
                                        .description("Arpajaistapahtuman id")
                                        .required(true)
                                        .kind(ApplicationCommandOptionType::Integer)
                                })
                        })
                        .create_option(|option| {
                                    option
                                        .name("delete")
                                        .kind(ApplicationCommandOptionType::SubCommand)
                                        .description("Poista arpajaistapahtuma")
                                        .create_sub_option(|subopt| {
                                            subopt
                                                .name("giveaway_id")
                                                .description("Arpajaistapahtuman viestin id")
                                                .required(true)
                                                .kind(ApplicationCommandOptionType::Integer)
                                        })
                                })

                });
                commands.create_application_command(|command| {
                    command
                        .name("⛔ Ilmianna viesti")
                        .kind(application_command::ApplicationCommandType::Message)
                })
            })
            .await
            .unwrap();
        for command in commands {
            if !command.default_permission {
                guild_id.set_application_commands_permissions(&ctx.http, |perms| {
                        perms.create_application_command(|command_perms| {
                            command_perms
                                .id(command.id.0)
                                .create_permissions(|cp| {
                                    cp
                                        .kind(serenity::model::interactions::application_command::ApplicationCommandPermissionType::Role)
                                        .id(admin_role_id)
                                        .permission(true)
                                })
                        })
                    })
                    .await
                    .unwrap();
            }
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::ApplicationCommand(ref a) => match a.data.name.as_ref() {
                "⛔ Ilmianna viesti" => voting::handle_report(&ctx, a.to_owned()).await,
                "github" => commands::links::github(&ctx, a.to_owned()).await,
                "giveaway" => commands::giveaway::handle_interaction(&ctx, a.to_owned()).await,
                _ => info!("Ignoring unknown interaction: `{}`", &a.data.name),
            },
            Interaction::MessageComponent(_) => {
                voting::handle_vote_interaction(&ctx, interaction.clone()).await;
                commands::giveaway::handle_component_interaction(&ctx, interaction.clone()).await;
            }
            _ => {}
        };
    }

    async fn message(&self, ctx: Context, msg: Message) {
        let db = ctx.get_db().await;

        // FIXME: Store in memory
        let words = match std::fs::read_to_string("blacklist.txt") {
            Ok(s) => s,
            Err(e) => {
                match e.kind() {
                    std::io::ErrorKind::NotFound => {
                        std::fs::File::create("blacklist.txt")
                            .expect("Unable to create blacklist.txt");
                    }
                    _ => panic!("Unable to access blacklist.txt"),
                }
                String::new()
            }
        };

        let mut data = ctx.data.write().await;
        let regexes = data.get_mut::<BlacklistRegexes>().unwrap();

        let last_edited = std::fs::metadata("blacklist.txt")
            .unwrap()
            .modified()
            .unwrap();

        if last_edited != regexes.lock().await.last_edited {
            info!("Generating new blacklist regexes");
            let mut new_vec = Vec::new();
            for w in words.lines() {
                if w.is_empty() {
                    continue;
                }
                if let Ok(r) = regex::Regex::new(w) {
                    new_vec.push(r);
                }
            }
            *regexes.lock().await = BlacklistRegexes {
                last_edited,
                regexvec: new_vec,
            };
        }

        for re in &regexes.lock().await.regexvec {
            if re.is_match(&msg.content) {
                msg.delete(&ctx.http).await.ok();
                return;
            }
        }

        if let Some(gid) = msg.guild_id {
            if gid == env::var("GUILD_ID").unwrap().parse::<u64>().unwrap() {
                db.increment_message_count(msg.author.id.as_u64())
                    .await
                    .ok();
            };
        }
    }

    async fn message_update(
        &self,
        ctx: Context,
        _: Option<Message>,
        _: Option<Message>,
        event: MessageUpdateEvent,
    ) {
        voting::handle_edit(&ctx, &event).await;
    }

    async fn message_delete(
        &self,
        ctx: Context,
        _: ChannelId,
        message_id: MessageId,
        _: Option<GuildId>,
    ) {
        voting::handle_delete(&ctx, message_id).await;
    }

    async fn guild_member_addition(&self, ctx: Context, _guild_id: GuildId, member: Member) {
        info!("{} joined", member.user);
        let member_role = env::var("MEMBER_ROLE_ID")
            .expect("member role id not found in $MEMBER_ROLE_ID")
            .parse::<u64>()
            .expect("Invalid member role id");
        member.clone().add_role(&ctx.http, member_role).await.ok();
    }

    async fn resume(&self, _: Context, _: ResumedEvent) {
        info!("Resumed");
    }
}

#[group]
#[commands(quit, award_ceremony)]
struct General;

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("Failed to load .env file");

    tracing_subscriber::fmt::init();

    let database = Database::new().await;

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let application_id = env::var("APPLICATION_ID")
        .expect("Expected an application id")
        .parse::<u64>()
        .expect("Invalid application id form");
    let http = Http::new_with_token(&token);

    let (owners, _bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            owners.insert(info.owner.id);

            (owners, info.id)
        }
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    let words = match std::fs::read_to_string("blacklist.txt") {
        Ok(s) => s,
        Err(e) => {
            match e.kind() {
                std::io::ErrorKind::NotFound => {
                    std::fs::File::create("blacklist.txt").expect("Unable to create blacklist.txt");
                }
                _ => panic!("Unable to access blacklist.txt"),
            }
            String::new()
        }
    };

    let mut regexvec = Vec::new();

    for w in words.lines() {
        if w.is_empty() {
            continue;
        }
        if let Ok(re) = regex::Regex::new(w) {
            regexvec.push(re);
        } else {
            info!("Skipping invalid regex in `blacklist.txt`: {}", w);
        }
    }

    let blacklist = BlacklistRegexes {
        last_edited: std::fs::metadata("blacklist.txt")
            .unwrap()
            .modified()
            .unwrap(),
        regexvec,
    };

    let framework = StandardFramework::new()
        .configure(|c| c.owners(owners).prefix("!"))
        .group(&GENERAL_GROUP);

    let mut client = Client::builder(&token)
        .application_id(application_id)
        .framework(framework)
        .event_handler(Handler)
        .intents(
            GatewayIntents::non_privileged()
                | GatewayIntents::GUILD_MEMBERS
                | GatewayIntents::GUILD_PRESENCES,
        )
        .await
        .expect("Err creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(client.shard_manager.clone());
        data.insert::<Database>(database);
        data.insert::<BlacklistRegexes>(Arc::new(Mutex::new(blacklist)));
    }

    let shard_manager = client.shard_manager.clone();

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let http = client.cache_and_http.http.clone();
    let db = Arc::new(client.get_db().await);

    let mut scheduler = Scheduler::with_tz(chrono::Local);

    for func in crate::scheduled::SETUP_FUNCTIONS {
        func(
            &mut scheduler,
            runtime.handle().to_owned(),
            http.clone(),
            db.clone(),
        );
    }

    let thread_handle = scheduler.watch_thread(std::time::Duration::from_millis(1000));

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        runtime.shutdown_background();
        thread_handle.stop();
        shard_manager.lock().await.shutdown_all().await;
    });

    if let Err(e) = client.start().await {
        error!("Client error: {}", e);
    }
}
