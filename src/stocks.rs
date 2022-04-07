use chrono::offset::Utc;
use chrono::{DateTime, TimeZone};
use chrono_tz;
use std::error::Error;
use yahoo_finance_api as yahoo;

trait QuoteTime {
    fn time(&self) -> DateTime<Utc>;
}

impl QuoteTime for yahoo::Quote {
    fn time(&self) -> DateTime<Utc> {
        Utc.timestamp(self.timestamp as i64, 0)
    }
}

pub async fn get_ticker_change_percentage(ticket: &str) -> Result<f64, Box<dyn Error>> {
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
