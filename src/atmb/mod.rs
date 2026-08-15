use color_eyre::eyre::Result;
use tracing::info;

pub mod model;
pub mod page;

use page::{CountryPage, LocationDetailPage, StatePage};
pub use model::{Mailbox, Address};

pub struct ATMBCrawl {
    client: reqwest::Client,
}

impl ATMBCrawl {
    // 完美纠正：去掉了可能导致 main 报错的 Result 包装，使其支持 ATMBCrawl::new() 直调
    pub fn new() -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "User-Agent",
            reqwest::header::HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        );
        Self {
            client: reqwest::Client::builder().default_headers(headers).build().unwrap(),
        }
    }

    pub async fn fetch_page(&self, url: String) -> Result<String> {
        let text = self.client.get(&url).send().await?.text().await?;
        Ok(text)
    }

    // 完美纠正：函数名改为 main.rs 死死认定的 `fetch` 动作
    pub async fn fetch(&self) -> Result<Vec<Mailbox>> {
        info!("Starting 2026 ATMB Mailbox spider...");

        let country_html = self.fetch_page("https://anytimemailbox.com".to_string()).await?;
        let country_page = CountryPage::parse_html(&country_html)?;
        let total_states = country_page.states.len();
        info!("Successfully found {total_states} US states.");

        let mut state_pages = Vec::new();
        for (idx, state_html_info) in country_page.states.iter().enumerate() {
            info!("[{}/{total_states}] fetching [{}] state page...", idx + 1, state_html_info.name());
            if let Ok(state_html) = self.fetch_page(state_html_info.url()).await {
                if let Ok(sp) = StatePage::parse_html(&state_html) {
                    state_pages.push(sp);
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }

        let total_num = state_pages.iter().map(|sp| sp.len()).sum::<usize>();
        info!("Successfully fetched {total_num} locations total.");

        let mut mailboxes = Vec::new();
        for sp in state_pages {
            if let Some(mut mbs) = sp.to_mailboxes() {
                mailboxes.append(&mut mbs);
            }
        }

        let total_mailboxes = mailboxes.len();
        for (idx, mailbox) in mailboxes.iter_mut().enumerate() {
            info!("[{}/{total_mailboxes}] fetching [{}] detail page...", idx + 1, mailbox.name);
            let detail_url = format!("https://anytimemailbox.com/{}/{}", mailbox.address.state.to_lowercase(), mailbox.link);
            if let Ok(html) = self.fetch_page(detail_url).await {
                if let Ok(detail_page) = LocationDetailPage::parse_html(&html) {
                    mailbox.address.line1 = detail_page.street();
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        }

        info!("Spider finished. Captured {} total mailboxes.", mailboxes.len());
        Ok(mailboxes)
    }
}
