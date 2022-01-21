use serenity::{
    async_trait, client,
    model::interactions::application_command::{
        ApplicationCommandInteractionDataOption, ApplicationCommandInteractionDataOptionValue,
    },
};

use crate::{database::Database, PartialChannel, PartialMember, Role, User};

#[async_trait]
pub trait ClientContextExt {
    async fn get_db(&self) -> Database;
}

pub trait InteractionDataOptionExt {
    fn to_string(&self) -> Option<String>;
    fn to_i64(&self) -> Option<i64>;
    fn to_bool(&self) -> Option<bool>;
    fn to_user(&self) -> Option<(User, Option<PartialMember>)>;
    fn to_role(&self) -> Option<Role>;
    fn to_channel(&self) -> Option<PartialChannel>;
    fn to_f64(&self) -> Option<f64>;
}

pub trait ApplicationCommandInteractionDataOptionVecExt {
    fn by_name(&self, name: &str) -> Option<&ApplicationCommandInteractionDataOption>;
}

#[async_trait]
impl ClientContextExt for client::Context {
    async fn get_db(&self) -> Database {
        self.data.read().await.get::<Database>().unwrap().clone()
    }
}

#[async_trait]
impl ClientContextExt for client::Client {
    async fn get_db(&self) -> Database {
        self.data.read().await.get::<Database>().unwrap().clone()
    }
}

impl<'a> InteractionDataOptionExt for &'a ApplicationCommandInteractionDataOption {
    fn to_string(&self) -> Option<String> {
        self.resolved.as_ref().and_then(|v| {
            if let ApplicationCommandInteractionDataOptionValue::String(x) = v {
                Some(x.to_owned())
            } else {
                None
            }
        })
    }

    fn to_i64(&self) -> Option<i64> {
        self.resolved.as_ref().and_then(|v| {
            if let ApplicationCommandInteractionDataOptionValue::Integer(x) = v {
                Some(x.to_owned())
            } else {
                None
            }
        })
    }

    fn to_bool(&self) -> Option<bool> {
        self.resolved.as_ref().and_then(|v| {
            if let ApplicationCommandInteractionDataOptionValue::Boolean(x) = v {
                Some(x.to_owned())
            } else {
                None
            }
        })
    }

    fn to_user(&self) -> Option<(User, Option<PartialMember>)> {
        self.resolved.as_ref().and_then(|v| {
            if let ApplicationCommandInteractionDataOptionValue::User(x, p) = v {
                Some((x.to_owned(), p.to_owned()))
            } else {
                None
            }
        })
    }

    fn to_role(&self) -> Option<Role> {
        self.resolved.as_ref().and_then(|v| {
            if let ApplicationCommandInteractionDataOptionValue::Role(x) = v {
                Some(x.to_owned())
            } else {
                None
            }
        })
    }

    fn to_channel(&self) -> Option<PartialChannel> {
        self.resolved.as_ref().and_then(|v| {
            if let ApplicationCommandInteractionDataOptionValue::Channel(x) = v {
                Some(x.to_owned())
            } else {
                None
            }
        })
    }

    fn to_f64(&self) -> Option<f64> {
        self.resolved.as_ref().and_then(|v| {
            if let ApplicationCommandInteractionDataOptionValue::Number(x) = v {
                Some(x.to_owned())
            } else {
                None
            }
        })
    }
}

impl ApplicationCommandInteractionDataOptionVecExt
    for Vec<ApplicationCommandInteractionDataOption>
{
    fn by_name(&self, name: &str) -> Option<&ApplicationCommandInteractionDataOption> {
        self.iter().find(|x| x.name == name)
    }
}
