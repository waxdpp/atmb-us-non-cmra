use color_eyre::eyre::Result;
use serde::{Deserialize, Serialize};
use tracing::info;

pub mod page;

use page::{CountryPage, LocationDetailPage, StatePage};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    pub line1: String,
    pub city: String,
    pub state: String,
    pub zip: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Mailbox {
    pub id: String,
    pub name: String,
    pub address: Address,
}

pub struct AtmbClient {
    client: reqwest::Client,
}

impl AtmbClient {
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

    pub async fn fetch_all_mailboxes(&self) -> Result<Vec<Mailbox>> {
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
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
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
            let detail_url = format!("https://anytimemailbox.com/{}/{}", mailbox.address.state.to_lowercase(), mailbox.id);
            if let Ok(html) = self.fetch_page(detail_url).await {
                if let Ok(detail_page) = LocationDetailPage::parse_html(&html) {
                    mailbox.address.line1 = detail_page.street();
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }

        info!("Spider finished. Captured {} total mailboxes.", mailboxes.len());
        Ok(mailboxes)
    }
}
