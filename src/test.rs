mod stocks;
use std::error::Error;

const TICKERS: &[&str] = &["^GSPC", "IWDA.AS", "AMC"];

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    for ticker in TICKERS {
        let change = stocks::get_ticker_change_percentage(ticker).await.unwrap();
        println!("{} changed by {}%", ticker, change);
    }

    Ok(())
}
