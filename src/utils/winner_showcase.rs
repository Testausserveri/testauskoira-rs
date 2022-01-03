use std::env;
use std::sync::Arc;

use serenity::http::client::Http;
use serenity::model::id::ChannelId;
use tracing::error;

use crate::database::Database;

async fn give_award_role(http: &Http, db: Arc<Database>, winner: u64) {
    let award_role_id: u64 = env::var("AWARD_ROLE_ID")
        .expect("No AWARD_ROLE_ID in .env")
        .parse()
        .expect("Invalid AWARD_ROLE_ID");

    let guild_id: u64 = env::var("GUILD_ID")
        .expect("Expected GUILD_ID in .env")
        .parse()
        .expect("Invalid GUILD_ID provided");

    let mut winner_member = http.get_member(guild_id, winner).await.unwrap();
    winner_member.add_role(http, award_role_id).await.unwrap();
    let previous_winner = db.get_most_active(1, 1).await.unwrap()[0].0;
    if previous_winner == winner {
        return;
    }
    let mut previous_winner_member = http.get_member(guild_id, previous_winner).await.unwrap();
    if (previous_winner_member
        .remove_role(http, award_role_id)
        .await)
        .is_ok()
    {
        info!("Removed role from previous winner {}", previous_winner);
    } else {
        info!("No previous winner");
    }
}

pub async fn display_winner(http: Arc<Http>, db: Arc<Database>) {
    let winners = db.get_most_active(5, 0).await.unwrap();
    let total_msgs = db.get_total_daily_messages().await.unwrap();

    let channel = ChannelId::from(
        env::var("AWARD_CHANNEL_ID")
            .unwrap()
            .parse::<u64>()
            .unwrap(),
    );

    let guild_id = channel
        .to_channel(http.clone())
        .await
        .unwrap()
        .guild()
        .unwrap()
        .guild_id;

    let futs = winners
        .into_iter()
        .map(|(member, msg_count)| {
            let member = guild_id.member(http.clone(), member);
            (member, msg_count)
        })
        .map(|(member_future, msg_count)| async move { (member_future.await, msg_count) });

    let tasks: Vec<_> = futs.map(|winner| tokio::spawn(winner)).collect();

    let winners: Vec<(_, _)> = futures::future::join_all(tasks)
        .await
        .into_iter()
        .map(|winner| winner.unwrap())
        .collect();

    let img_name = super::build_award::build_award_image(&winners[0].0.as_ref().unwrap().face())
        .await
        .unwrap();

    give_award_role(&http, db.clone(), winners[0].0.as_ref().unwrap().user.id.0).await;

    channel
        .send_message(http.clone(), |m| {
            m.add_file(std::path::Path::new(&img_name));
            m.embed(|e| {
                e.title("Most active members");
                e.image(format!("attachment://{}", img_name));
                winners
                    .iter()
                    .enumerate()
                    .for_each(|(ranking, (member, msg_count))| {
                        let msg_percent = msg_count.to_owned() as f64 / total_msgs as f64 * 100.;
                        match member {
                            Ok(m) => {
                                e.field(
                                    format!("Number {}.", ranking),
                                    format!("{}, {} messages {:.2}%", m, msg_count, msg_percent),
                                    false,
                                );
                            }
                            Err(err) => {
                                e.field(
                                    format!("Number {}.", ranking),
                                    format!(
                                        "Former member, {} messages {:.2}%",
                                        msg_count, msg_percent
                                    ),
                                    false,
                                );
                                error!("{}", err);
                            }
                        };
                    });
                e
            })
        })
        .await
        .unwrap();
}
