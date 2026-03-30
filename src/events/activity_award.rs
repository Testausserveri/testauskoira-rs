use std::{io::Cursor, sync::Arc};

use futures::prelude::*;
use poise::serenity_prelude::{
    ChannelId, Colour, CreateAttachment, CreateEmbed, CreateMessage, Http,
};
use tracing::error;

use crate::database::Database;

async fn give_award_role(http: &Http, db: Arc<Database>, winner: u64) {
    let guild_id = poise::serenity_prelude::GuildId::new(crate::config::CONFIG.guild_id);
    let award_role_id = poise::serenity_prelude::RoleId::new(crate::config::CONFIG.award_role_id);

    if let Ok(previous_winner) = db.get_last_winner().await {
        if let Ok(member) = http
            .get_member(
                guild_id,
                poise::serenity_prelude::UserId::new(previous_winner),
            )
            .await
        {
            member.remove_role(http, award_role_id).await.ok();
        } else {
            info!("Cannot get the member info of the previous winner");
        }
    } else {
        info!("No previous winner found");
    }
    let winner_member = http
        .get_member(guild_id, poise::serenity_prelude::UserId::new(winner))
        .await
        .unwrap();
    winner_member.add_role(http, award_role_id).await.unwrap();
    db.new_winner(winner).await.ok();
}

pub async fn display_winner(http: Arc<Http>, db: Arc<Database>, offset: i32) {
    let db = db;
    let winners = db.get_most_active(5, offset).await.unwrap();
    let total_msgs = db.get_total_daily_messages(offset).await.unwrap();
    let messages_average = db.get_total_message_average(offset).await.unwrap();

    let channel = ChannelId::new(crate::config::CONFIG.award_channel_id);

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

    let build_embed = |winners: &Vec<(
        Result<poise::serenity_prelude::Member, poise::serenity_prelude::Error>,
        i32,
    )>| {
        let mut e = CreateEmbed::new()
            .title("Eilisen aktiivisimmat jäsenet")
            .description(format!(
                "Eilen lähetettin **{}** viestiä, joka on **{:.0} %** keskimääräisestä",
                &total_msgs,
                total_msgs as f32 / messages_average * 100f32
            ))
            .colour(Colour::from_rgb(68, 82, 130));

        for (ranking, (member, msg_count)) in winners.iter().enumerate() {
            let msg_percent = msg_count.to_owned() as f64 / total_msgs as f64 * 100.;
            match member {
                Ok(m) => {
                    e = e.field(
                        format!("Sijalla {}.", ranking),
                        format!("{}, {} viestiä ({:.1} %)", m, msg_count, msg_percent),
                        false,
                    );
                }
                Err(err) => {
                    e = e.field(
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
        }
        e
    };

    match &winners[0].0.as_ref() {
        Ok(winner) => {
            let img_data = build_award_image(&winner.face()).await;

            give_award_role(
                &http,
                db.clone(),
                winners[0].0.as_ref().unwrap().user.id.get(),
            )
            .await;

            let embed = build_embed(&winners);

            let mut msg_builder = CreateMessage::new();
            if let Ok(ref data) = img_data {
                let attachment = CreateAttachment::bytes(data.as_slice(), "pfp_new.png");
                msg_builder = msg_builder.add_file(attachment);
                let embed = embed.image("attachment://pfp_new.png");
                msg_builder = msg_builder.embed(embed);
            } else {
                msg_builder = msg_builder.embed(embed);
            }

            channel.send_message(&http, msg_builder).await.unwrap();
        }
        Err(_) => {
            let embed = build_embed(&winners);
            channel
                .send_message(&http, CreateMessage::new().embed(embed))
                .await
                .unwrap();
        }
    };
}

pub async fn build_award_image(user_img_url: &str) -> Result<Vec<u8>, anyhow::Error> {
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

    let mut buf = Vec::new();
    image::DynamicImage::ImageRgba8(pfp)
        .write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)?;

    Ok(buf)
}
