use std::{env, io::Cursor, sync::Arc};

use futures::prelude::*;
use serenity::{http::client::Http, model::id::ChannelId};
use tracing::error;

use crate::database::Database;

async fn give_award_role(http: &Http, db: Arc<Database>, winner: u64, offset: i32) {
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
    let yesterdays_competition = db.get_most_active(1, offset + 1).await.unwrap();
    if yesterdays_competition.is_empty() {
        return;
    }
    let previous_winner = yesterdays_competition[0].0;
    if previous_winner == winner {
        return;
    }
    let mut previous_winner_member = http.get_member(guild_id, previous_winner).await.unwrap();
    if (previous_winner_member
        .remove_role(http, award_role_id)
        .await)
        .is_ok()
    {
        info!(
            "Rooli poistettu edelliseltä voittajalta {}",
            previous_winner
        );
    } else {
        info!("Ei aiempaa voittajaa");
    }
}

pub async fn display_winner(http: Arc<Http>, db: Arc<Database>, offset: i32) {
    let db = db;
    let winners = db.get_most_active(5, offset).await.unwrap();
    let total_msgs = db.get_total_daily_messages(offset).await.unwrap();
    let messages_average = db.get_total_message_average(offset).await.unwrap();

    let channel = ChannelId::from(
        env::var("AWARD_CHANNEL_ID")
            .unwrap()
            .parse::<u64>()
            .unwrap(),
    );

    let guild_id = channel
        .to_channel(&http)
        .await
        .unwrap()
        .guild()
        .unwrap()
        .guild_id;

    let winners = stream::iter(winners)
        .map(|(member, msg_count)| {
            let future = guild_id.member(&http, member);
            async move { (future.await, msg_count) }
        })
        .buffered(5)
        .collect::<Vec<_>>()
        .await;

    match &winners[0].0.as_ref() {
        Ok(winner) => {
            let img_name = build_award_image(&winner.face()).await.unwrap();

            give_award_role(
                &http,
                db.clone(),
                winners[0].0.as_ref().unwrap().user.id.0,
                offset,
            )
            .await;

            channel
                .send_message(&http, |m| {
                    m.add_file(std::path::Path::new(&img_name));
                    m.embed(|e| {
                        e.title("Eilisen aktiivisimmat jäsenet");
                        e.description(format!(
                            "Eilen lähetettin **{}** viestiä, joka on **{:.0} %** keskimääräisestä",
                            &total_msgs,
                            total_msgs as f32 / messages_average * 100f32
                        ));
                        e.color(serenity::utils::Color::from_rgb(68, 82, 130));
                        e.image(format!("attachment://{}", img_name));
                        winners
                            .iter()
                            .enumerate()
                            .for_each(|(ranking, (member, msg_count))| {
                                let msg_percent =
                                    msg_count.to_owned() as f64 / total_msgs as f64 * 100.;
                                match member {
                                    Ok(m) => {
                                        e.field(
                                            format!("Sijalla {}.", ranking),
                                            format!(
                                                "{}, {} viestiä ({:.1} %)",
                                                m, msg_count, msg_percent
                                            ),
                                            false,
                                        );
                                    }
                                    Err(err) => {
                                        e.field(
                                            format!("Sijalla {}.", ranking),
                                            format!(
                                                "Entinen jäsen, {} viestiä ({:.1} %)",
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
        Err(_) => {
            channel
                .send_message(&http, |m| {
                    m.embed(|e| {
                        e.title("Eilisen aktiivisimmat jäsenet");
                        e.description(format!(
                            "Eilen lähetettin **{}** viestiä, joka on **{:.0} %** keskimääräisestä",
                            &total_msgs,
                            total_msgs as f32 / messages_average * 100f32
                        ));
                        e.color(serenity::utils::Color::from_rgb(68, 82, 130));
                        winners
                            .iter()
                            .enumerate()
                            .for_each(|(ranking, (member, msg_count))| {
                                let msg_percent =
                                    msg_count.to_owned() as f64 / total_msgs as f64 * 100.;
                                match member {
                                    Ok(m) => {
                                        e.field(
                                            format!("Sijalla {}.", ranking),
                                            format!(
                                                "{}, {} viestiä ({:.1} %)",
                                                m, msg_count, msg_percent
                                            ),
                                            false,
                                        );
                                    }
                                    Err(err) => {
                                        e.field(
                                            format!("Sijalla {}.", ranking),
                                            format!(
                                                "Entinen jäsen, {} viestiä ({:.1} %)",
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
    };
}

pub async fn build_award_image(user_img_url: &str) -> Result<String, ()> {
    let img_url_base = &user_img_url[..user_img_url.rfind('.').unwrap()];
    let profile_picture = reqwest::get(format!("{}.png?size=128", img_url_base))
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    let pfp = image::io::Reader::new(Cursor::new(profile_picture))
        .with_guessed_format()
        .unwrap()
        .decode()
        .unwrap();
    let mask = image::io::Reader::open("img/blackcomposite.png")
        .unwrap()
        .decode()
        .unwrap();

    let mut pfp = pfp.to_rgba8();
    let mask = mask.to_rgba8();

    for (x, y, pixel) in pfp.enumerate_pixels_mut() {
        let mask_pixel = mask.get_pixel(x, y);
        if mask_pixel[3] < 150 {
            *pixel = *mask_pixel;
        }
    }

    image::imageops::overlay(&mut pfp, &mask, 0, 0);
    pfp.save("pfp_new.png").unwrap();

    Ok("pfp_new.png".to_string())
}
