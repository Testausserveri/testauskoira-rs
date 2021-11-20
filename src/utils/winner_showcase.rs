use twilight_http::{request::guild::member::GetMember,Client};
use crate::database::Database;
use std::{env, sync::Arc,fs::File,io::{BufReader,Read}};
use tracing::error;
use twilight_model::{id::{ChannelId,GuildId,UserId},channel::{GuildChannel,embed::{Embed,EmbedField,EmbedImage}}};
use twilight_embed_builder::{EmbedBuilder, EmbedFieldBuilder, image_source::ImageSource};

pub async fn display_winner(http: Arc<Client>, db: Arc<Database>) {
    let winners = db.get_most_active(5).await.unwrap();
    let total_msgs = db.get_total_daily_messages().await.unwrap();

    let channel = ChannelId::new(
        env::var("AWARD_CHANNEL_ID")
            .unwrap()
            .parse::<u64>()
            .unwrap(),
    ).unwrap();

    let guild_id = GuildId::new(
        env::var("GUILD_ID")
            .expect("Missing guild id")
            .parse::<u64>()
            .unwrap()
    ).unwrap();

    let futs = winners
        .into_iter()
        .map(|(member_id, msg_count)| {
            let member = http.guild_member(guild_id,UserId::new(member_id).unwrap()).exec();
            (member, msg_count)
        })
        .map(|(member_future, msg_count)| async move { (member_future.await, msg_count) });

    let tasks: Vec<_> = futs.map(|winner| tokio::spawn(winner)).collect();

    let winners: Vec<(_, _)> = futures::future::join_all(tasks)
        .await
        .into_iter()
        .map(|winner| winner.unwrap())
        .collect();

    let futs = winners
        .into_iter()
        .map(|(member,msg_count)| async move { ( member.unwrap().model().await,msg_count) });
    
    let tasks: Vec<_> = futs.map(|winner| tokio::spawn(winner)).collect();

    let winners: Vec<_> = futures::future::join_all(tasks)
        .await
        .into_iter()
        .map(|winner| winner.unwrap())
        .map(|(m,c)| (m.unwrap(),c))
        .collect();

    let img_name = super::build_award::build_award_image(winners[0].0.user.id.get(),&winners[0].0.clone().user.avatar.unwrap())
        .await
        .unwrap();

    let winners = winners
        .iter()
        .enumerate()
        .map(|(ranking, (member, msg_count))| {
            let msg_percent = *msg_count as f64 / total_msgs as f64 * 100.;
            (ranking,member,msg_count,msg_percent)
        })
        .collect::<Vec<_>>();

    let mut embed = EmbedBuilder::new();
    embed = embed.image(twilight_embed_builder::image_source::ImageSource::attachment("pfp_new.png").unwrap());
    for (ranking,member,msg_count,msg_percent) in winners {
        embed = embed.field(
            EmbedFieldBuilder::new(
                format!("Number {}.", ranking),
                format!("{}, {} messages {:.2}%", member.user.name, msg_count, msg_percent),
            ).build()
        );
    }
    let embed = embed.build().unwrap();

    let f = File::open("pfp_new.png").unwrap();
    let mut reader = BufReader::new(f);
    let mut buffer = Vec::new();
    
    // Read file into vector.
    reader.read_to_end(&mut buffer).unwrap();

    http.create_message(channel)
        .files(&[("pfp_new.png",buffer.as_slice())])
        .embeds(&[embed])
        .unwrap()
        .exec()
        .await
        .unwrap();
}
