use actix_web::http::header;
use actix_web::{dev::{Server,ServerHandle}, web, App, HttpResponse, HttpServer};
use oauth2::basic::BasicClient;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, TokenResponse,
    TokenUrl,
};

use twilight_model::id::GuildId;

use crate::Database;

use std::{collections::HashMap, env, sync::Arc};

use reqwest::header::{AUTHORIZATION, USER_AGENT};

use twilight_http::Client;

struct AppState {
    oauth: BasicClient,
    http: Arc<Client>,
    db: Arc<Database>,
}

fn index(data: web::Data<Arc<AppState>>) -> HttpResponse {
    let (auth_url, _csrf_token) = &data.oauth.authorize_url(CsrfToken::new_random).url();

    HttpResponse::Found()
        .append_header((header::LOCATION, auth_url.to_string()))
        .finish()
}

#[derive(Deserialize)]
struct AuthRequest {
    code: String,
    state: String,
}

#[derive(Deserialize)]
struct User {
    login: String,
    id: u64,
}

async fn guild_info(data: web::Data<Arc<AppState>>) -> HttpResponse {
    let guild_id: u64 = std::env::var("GUILD_ID").unwrap().parse().unwrap();
    let msg_count = data.db.get_total_daily_messages().await.unwrap();
    let guild = data.http.guild(GuildId::new(guild_id).expect("Invalid guild id")).exec().await.unwrap().model().await.unwrap();
    let members = guild.member_count.unwrap();

    let mut map: HashMap<&str, u64> = HashMap::new();
    map.insert("memberCount", members as u64);
    map.insert("messagesToday", msg_count);

    HttpResponse::Ok().json(map)
}

async fn auth(data: web::Data<Arc<AppState>>, params: web::Query<AuthRequest>) -> HttpResponse {
    let pat = std::env::var("PAT").unwrap();
    let org = std::env::var("ORG_NAME").unwrap();

    let code = AuthorizationCode::new(params.code.clone());
    let _state = CsrfToken::new(params.state.clone());

    let token: _ = &data
        .oauth
        .exchange_code(code)
        // NOTE: Using curl here because reqwest did not work for unknown reasons
        .request(oauth2::curl::http_client)
        .expect("exchange_code failed");

    let url = "https://api.github.com/user";

    let client = reqwest::Client::new();
    let access_token = token.access_token().secret();

    let resp = client
        .get(url)
        .header(USER_AGENT, "testaukoira-rs")
        .header(AUTHORIZATION, format!("token {}", access_token))
        .send()
        .await
        .unwrap();

    let user = resp.json::<User>().await.unwrap();

    let mut map = HashMap::new();
    map.insert("invitee_id", user.id);

    let join_resp = client
        .post(format!("https://api.github.com/orgs/{}/invitations", org))
        .header(USER_AGENT, "testaukoira-rs")
        .header(AUTHORIZATION, format!("token {}", pat))
        .json(&map)
        .send()
        .await
        .unwrap();

    match join_resp.error_for_status() {
        Ok(_) => {
            let mut map = HashMap::new();
            map.insert("accept", "application/vnd.github.v3+json");
            map.insert("state", "active");

            client
                .patch(format!(
                    "https://api.github.com/user/memberships/orgs/{}",
                    org
                ))
                .header(USER_AGENT, "testaukoira-rs")
                .header(AUTHORIZATION, format!("token {}", access_token))
                .send()
                .await
                .unwrap();

            client
                .put(format!(
                    "https://api.github.com/orgs/{}/public_members/{}",
                    org, user.login
                ))
                .header(USER_AGENT, "testaukoira-rs")
                .header(AUTHORIZATION, format!("token {}", access_token))
                .send()
                .await
                .unwrap();

            let channel = twilight_model::id::ChannelId::new(880127231664459809).unwrap();
            data.http.create_message(channel)
                .content(format!("{} liittyi Testausserverin GitHub-organisaatioon 🎉! Liity sinäkin: <https://koira.testausserveri.fi/github/join>",user.login).as_str())
                .unwrap()
                .exec()
                .await
                .unwrap();
        }
        Err(e) => error!("{}", e),
    }

    HttpResponse::Found()
        .append_header((header::LOCATION, format!("https://github.com/{}", org)))
        .finish()
}

pub async fn start_api(http: Arc<Client>, db: Arc<Database>) -> ServerHandle {
    dotenv::dotenv().expect("Failed to load .env file");

    HttpServer::new(move || {
        let client_id = ClientId::new(
            env::var("CLIENT_ID").expect("Missing the CLIENT_ID environment variable."),
        );
        let client_secret = ClientSecret::new(
            env::var("CLIENT_SECRET").expect("Missing the CLIENT_SECRET environment variable."),
        );

        let auth_url = AuthUrl::new(String::from("https://github.com/login/oauth/authorize"))
            .expect("Invalid authorization endpoint URL");
        let token_url = TokenUrl::new(String::from("https://github.com/login/oauth/access_token"))
            .expect("Invalid token endpoint URL");

        let client = BasicClient::new(client_id, Some(client_secret), auth_url, Some(token_url))
            .set_redirect_uri(
                RedirectUrl::new("http://localhost:8080/api/authorized".to_string())
                    .expect("Invalid redirect URL"),
            );

        let data = web::Data::new(Arc::new(AppState {
            oauth: client,
            http: http.clone(),
            db: db.clone(),
        }));

        App::new()
            .app_data(data)
            .route("/github/join", web::get().to(index))
            .route("/api/authorized", web::get().to(auth))
            .route("/api/guildInfo", web::get().to(guild_info))
    })
    .bind("localhost:8080")
    .expect("Can not bind to port 8080")
    .run()
    .handle()
}
