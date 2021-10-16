use serenity::{model::{guild::Member,id::{GuildId,ChannelId}},http::client::Http};
use crate::database::Database;
use std::sync::Arc;

pub async fn display_winner(http: Arc<Http>,db: Arc<Database>) {
    let winners = db.get_most_active(5).await.unwrap();
    
    let httpclone = http.clone();

    let futs = winners.into_iter().map(|w| {
        let (w,m) = w;
        let guild = GuildId::from(880127231664459806);
        let member= guild.member(httpclone.clone(),w.clone());
        (member,m)
    }).map(|w| async {
        let (w,m) = w;
        (w.await,m)
    });

    let tasks: Vec<_> = futs.map(|w| {
        tokio::spawn(w) 
    }).collect();

    let winners: Vec<(Member,i32)> = futures::future::join_all(tasks)
        .await
        .into_iter()
        .map(|w| { let (w,c) = w.unwrap(); (w.unwrap(),c) })
        .collect::<Vec<_>>();

    let img_name = super::build_award::build_award_image(&winners[0].0.face()).await.unwrap();

    ChannelId::from(880127231664459809).send_message(http.clone(),|m| {
        m.add_file(std::path::Path::new(&img_name));        
        m.embed(|e| {
            e.title("Most active members");
            e.image(format!("attachment://{}",img_name));
            winners.iter().enumerate().for_each(|(i,(w,c))| {
                e.field(format!("Number {}.",i),format!("{}, {} messages",w,c),false);
            });
            e
        })
    }).await.unwrap();
}
