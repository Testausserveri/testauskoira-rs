use actix_web::http::header;
use actix_web::{dev::Server, web, App, HttpResponse, HttpServer};
use oauth2::basic::BasicClient;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, TokenResponse,
    TokenUrl,
};

use std::{collections::HashMap, env, sync::Arc};

use reqwest::header::{AUTHORIZATION, USER_AGENT};

use serenity::http::Http;

struct AppState {
    oauth: BasicClient,
    http: Arc<Http>,
}

fn index(data: web::Data<Arc<AppState>>) -> HttpResponse {
    let (auth_url, _csrf_token) = &data.oauth.authorize_url(CsrfToken::new_random).url();

    println!("{}", auth_url);

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

async fn auth(data: web::Data<Arc<AppState>>, params: web::Query<AuthRequest>) -> HttpResponse {
    let pat = std::env::var("PAT").unwrap();

    let code = AuthorizationCode::new(params.code.clone());
    let _state = CsrfToken::new(params.state.clone());

    let token: _ = &data
        .oauth
        .exchange_code(code)
        // NOTE: Using curl here because reqwest did not work for unknown reasons
        .request(oauth2::curl::http_client)
        .expect("exchange_code failed");

    println!("Token: {:#?}", token.access_token());

    let url = "https://api.github.com/user";

    let client = reqwest::Client::new();
    let access_token = token.access_token().secret();

    let resp = client
        .get(url)
        .header(USER_AGENT, "testaukoira-rs")
        .header(
            AUTHORIZATION,
            format!("token {}", access_token),
        )
        .send()
        .await
        .unwrap();

    let user = resp.json::<User>().await.unwrap();

    let mut map = HashMap::new();
    map.insert("invitee_id", user.id);

    let join_resp = client
        .post("https://api.github.com/orgs/vilepis-testing-org/invitations")
        .header(USER_AGENT, "testaukoira-rs")
        .header(AUTHORIZATION, format!("token {}", pat))
        .json(&map)
        .send()
        .await
        .unwrap();

    let mut map = HashMap::new();
    map.insert("accept", "application/vnd.github.v3+json");
    map.insert("state", "active");

    client
        .patch("https://api.github.com/user/memberships/orgs/Testausserveri")
        .header(USER_AGENT, "testaukoira-rs")
        .header(AUTHORIZATION, format!("token {}", access_token))
        .send()
        .await
        .unwrap();

    client
        .put(format!("https://api.github.com/orgs/Testausserveri/public_members/{}",user.login))
        .header(USER_AGENT, "testaukoira-rs")
        .header(AUTHORIZATION, format!("token {}", access_token ))
        .send()
        .await
        .unwrap();

    serenity::model::id::ChannelId::from(880127231664459809)
        .say(data.http.clone(),format!("{} liittyi Testausserverin GitHub-organisaatioon 🎉! Liity sinäkin: https://koira.testausserveri.fi/github/join",user.login))
        .await
        .ok();

    HttpResponse::Found()
        .append_header((header::LOCATION, "https://github.com/Testausserveri"))
        .finish()
}

pub async fn start_api(http: Arc<serenity::http::client::Http>) -> Server {
    dotenv::dotenv().expect("Failed to load .env file");

    HttpServer::new(move || {
        let client_id = ClientId::new(
            env::var("CLIENT_ID").expect("Missing the CLIENT_ID environment variable."),
        );
        let client_secret = ClientSecret::new(
            env::var("CLIENT_SECRET").expect("Missing the CLIENT_SECRET environment variable."),
        );

        let oauthserver = env::var("SERVER").expect("Missing the SERVER environment variable.");
        let auth_url = AuthUrl::new(format!("https://{}/login/oauth/authorize", oauthserver))
            .expect("Invalid authorization endpoint URL");
        let token_url = TokenUrl::new(format!("https://{}/login/oauth/access_token", oauthserver))
            .expect("Invalid token endpoint URL");

        let client = BasicClient::new(client_id, Some(client_secret), auth_url, Some(token_url))
            .set_redirect_uri(
                RedirectUrl::new("http://localhost:8080/api/authorized".to_string())
                    .expect("Invalid redirect URL"),
            );

        let data = web::Data::new(Arc::new(AppState {
            oauth: client,
            http: http.clone(),
        }));

        App::new()
            .app_data(data)
            .route("/", web::get().to(index))
            .route("/api/authorized", web::get().to(auth))
    })
    .bind("localhost:8080")
    .expect("Can not bind to port 8080")
    .run()
}
