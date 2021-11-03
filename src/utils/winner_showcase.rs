use crate::database::Database;
use serenity::{http::client::Http, model::id::ChannelId};
use std::env;
use std::sync::Arc;
use tracing::error;

pub async fn display_winner(http: Arc<Http>, db: Arc<Database>) {
    let winners = db.get_most_active(5).await.unwrap();
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
