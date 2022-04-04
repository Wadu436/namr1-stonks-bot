use chrono::offset::Utc;
use chrono::{DateTime, TimeZone};

use chrono_tz;

use dotenv::dotenv;

use serenity::async_trait;
use serenity::client::{Client, Context, EventHandler};
use serenity::http::CacheHttp;
use serenity::model::{
    guild::Guild,
    id::{GuildId, RoleId, UserId},
    prelude::Ready,
};

use std::env;
use std::error::Error;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use tokio::time::{self, MissedTickBehavior};

use yahoo_finance_api as yahoo;

struct Handler {
    is_loop_running: AtomicBool,
}

const JAPANESE_GREEN_ID: RoleId = RoleId(621894745807126538);
const JAPANESE_RED_ID: RoleId = RoleId(621894973360439299);
const NAMR1_GUILD_ID: GuildId = GuildId(286572805137498112);
const USER_ID_TICKER: &[(UserId, &str)] = &[
    (UserId(178070915542810624), "^GSPC"), // Charles
    (UserId(168355107396780032), "AMC"),   // Nam
];

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }

    async fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
        println!("Cache built successfully!");

        let ctx = Arc::new(ctx);
        if !self.is_loop_running.load(Ordering::Relaxed) {
            let ctx = Arc::clone(&ctx);
            tokio::spawn(role_task_repeat(ctx));
            self.is_loop_running.swap(true, Ordering::Relaxed);
        }
    }
}

trait QuoteTime {
    fn time(&self) -> DateTime<Utc>;
}

impl QuoteTime for yahoo::Quote {
    fn time(&self) -> DateTime<Utc> {
        Utc.timestamp(self.timestamp as i64, 0)
    }
}

async fn get_ticker_change_percentage(ticket: &str) -> Result<f64, Box<dyn Error>> {
    let yahoo_conn = yahoo::YahooConnector::new();
    let quote_res = yahoo_conn.get_quote_range(ticket, "1d", "5d").await?;

    let quotes = quote_res.quotes()?;

    if quotes.len() == 0 {
        Err("No quotes were found")?;
    }

    let last_quote = &quotes[quotes.len() - 1];
    let last_quote_date = last_quote
        .time()
        .with_timezone(&chrono_tz::US::Eastern)
        .date();
    let mut previous_quote: Option<&yahoo::Quote> = None;

    // Find previous quote
    for i in (0..quotes.len() - 1).rev() {
        let delta = last_quote_date
            - quotes[i]
                .time()
                .with_timezone(&chrono_tz::US::Eastern)
                .date();
        if delta >= chrono::Duration::days(1) {
            previous_quote = Some(&quotes[i]);
            break;
        }
    }

    if previous_quote.is_none() {
        Err("Error: Couldn't find a quote from 1 day ago.")?;
    }
    let previous_quote = previous_quote.unwrap();

    let last_close = previous_quote.close;
    let current_close = last_quote.close;
    let delta = current_close - last_close;
    let pcntg = 100. * (delta / last_close);

    return Ok(pcntg);
}

async fn role_task_repeat(ctx: Arc<Context>) {
    // let mut long_interval = time::interval(Duration::from_secs(1 * 24 * 60 * 60)); // Long delay, once per day
    let mut long_interval = time::interval(Duration::from_secs(60)); // Long delay, every minute
    let mut short_interval = time::interval(Duration::from_secs(5)); // Short delay for when we encounter an error

    long_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    short_interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

    // Use initial ticks
    long_interval.tick().await;
    short_interval.tick().await;

    loop {
        for (id, ticker) in USER_ID_TICKER {
            let mut retry_count = 3;
            while retry_count > 0 {
                if let Err(why) = change_role_task(ctx.clone(), id, ticker).await {
                    eprintln!("{}", why);
                } else {
                    break;
                }
                retry_count -= 1;

                short_interval.tick().await;
            }
        }

        long_interval.tick().await;
    }
}

async fn change_role_task(
    ctx: Arc<Context>,
    id: &UserId,
    ticker: &str,
) -> Result<(), Box<dyn Error>> {
    let cache_http = (&ctx.cache, ctx.http());
    let guild = Guild::get(&ctx.http, NAMR1_GUILD_ID).await?;

    let japanese_red = guild
        .roles
        .get(&JAPANESE_RED_ID)
        .ok_or("Couldn't find role Japanese Red")?;
    let japanese_green = guild
        .roles
        .get(&JAPANESE_GREEN_ID)
        .ok_or("Couldn't find role Japanese Green")?;

    let change = get_ticker_change_percentage(ticker).await?;

    let mut member = guild.member(cache_http, id).await?;

    let (set_role, unset_role) = if change > 0.0 {
        (japanese_red, japanese_green)
    } else {
        (japanese_green, japanese_red)
    };

    if !member.roles.contains(&set_role.id) {
        member.add_role(cache_http, set_role).await?;
    }

    if member.roles.contains(&unset_role.id) {
        member.remove_role(cache_http, unset_role).await?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    // Login with a bot token from the environment
    let token = env::var("DISCORD_TOKEN").expect("token");
    let mut client = Client::builder(token)
        .event_handler(Handler {
            is_loop_running: AtomicBool::new(false),
        })
        .await
        .expect("Error creating client");

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }

    Ok(())
}
