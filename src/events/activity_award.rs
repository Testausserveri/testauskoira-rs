use std::{env, io::Cursor, sync::Arc};

use futures::prelude::*;
use serenity::{http::client::Http, model::id::ChannelId};
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

    if let Ok(previous_winner) = db.get_last_winner().await {
        if let Ok(mut member) = http.get_member(guild_id, previous_winner).await {
            member.remove_role(http, award_role_id).await.ok();
        } else {
            info!("Cannot get the member info of the previous winner");
        }
    } else {
        info!("No previous winner found");
    }
    let mut winner_member = http.get_member(guild_id, winner).await.unwrap();
    winner_member.add_role(http, award_role_id).await.unwrap();
    db.new_winner(winner).await.ok();
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
            let img_name = build_award_image(&winner.face()).await;

            give_award_role(&http, db.clone(), winners[0].0.as_ref().unwrap().user.id.0).await;

            channel
                .send_message(&http, |m| {
                    if img_name.is_ok() {
                        m.add_file(std::path::Path::new(img_name.as_ref().unwrap()));
                    }
                    m.embed(|e| {
                        e.title("Eilisen aktiivisimmat jäsenet");
                        e.description(format!(
                            "Eilen lähetettin **{}** viestiä, joka on **{:.0} %** keskimääräisestä",
                            &total_msgs,
                            total_msgs as f32 / messages_average * 100f32
                        ));
                        e.color(serenity::utils::Color::from_rgb(68, 82, 130));
                        if img_name.is_ok() {
                            e.image(format!("attachment://{}", &img_name.as_ref().unwrap()));
                        }
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

pub async fn build_award_image(user_img_url: &str) -> Result<String, anyhow::Error> {
    let img_url_base = &user_img_url[..user_img_url.rfind('.').unwrap()];
    let profile_picture = reqwest::get(format!("{}.png?size=128", img_url_base))
        .await?
        .bytes()
        .await?;
    let pfp = image::io::Reader::new(Cursor::new(profile_picture))
        .with_guessed_format()?
        .decode()?
        .resize(128, 128, image::imageops::FilterType::Gaussian);
    let mask = image::io::Reader::open("img/blackcomposite.png")?.decode()?;

    let mut pfp = pfp.to_rgba8();
    let mask = mask.to_rgba8();

    for (x, y, pixel) in pfp.enumerate_pixels_mut() {
        let mask_pixel = mask.get_pixel(x, y);
        if mask_pixel[3] < 150 {
            *pixel = *mask_pixel;
        }
    }

    image::imageops::overlay(&mut pfp, &mask, 0, 0);
    pfp.save("pfp_new.png")?;

    Ok("pfp_new.png".to_string())
}
