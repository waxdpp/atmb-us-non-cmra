use color_eyre::eyre::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

pub mod page;

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

    // 核心骨架：一步到位直接在云端抓取所有最新地址
    pub async fn fetch_all_mailboxes(&self) -> Result<Vec<Mailbox>> {
        info!("Starting 2026 ATMB Mailbox spider...");
        let mut mailboxes = Vec::new();

        // 1. 直接抓取全美主页面
        let country_url = "https://anytimemailbox.com";
        let country_html = self.client.get(country_url).send().await?.text().await?;
        
        let states = page::parse_states(&country_html)?;
        let total_states = states.len();
        info!("Successfully found {total_states} US states.");

        // 2. 遍历各州抓取网点 (为了展示进度和节省额度，只抓取核心有地址的州)
        for (idx, state) in states.iter().enumerate() {
            info!("[{}/{total_states}] Fetching state: [{}]", idx + 1, state.name);
            let state_url = format!("https://anytimemailbox.com/{}", state.slug);
            
            let state_html = match self.client.get(&state_url).send().await {
                Ok(resp) => match resp.text().await {
                    Ok(text) => text,
                    Err(_) => continue,
                },
                Err(_) => continue,
            };

            let locations = match page::parse_locations(&state_html) {
                Ok(locs) => locs,
                Err(_) => continue,
            };

            // 3. 提取地址并生成标准的 Mailbox 格式数据
            for loc in locations {
                let detail_url = format!("https://anytimemailbox.com/{}/{}", state.slug, loc.slug);
                let detail_html = match self.client.get(&detail_url).send().await {
                    Ok(resp) => match resp.text().await {
                        Ok(text) => text,
                        Err(_) => continue,
                    },
                    Err(_) => continue,
                };

                if let Ok(addrs) = page::parse_addresses(&detail_html) {
                    if let Some(raw_addr) = addrs.first() {
                        // 智能切分结构化地址
                        let mut mailbox = Mailbox {
                            id: loc.slug.clone(),
                            name: loc.name.clone(),
                            ..Default::default()
                        };
                        
                        mailbox.address.line1 = raw_addr.clone();
                        mailbox.address.state = state.slug.to_uppercase();
                        
                        let parts: Vec<&str> = raw_addr.split_whitespace().collect();
                        if let Some(last) = parts.last() {
                            if last.len() == 5 && last.chars().all(|c| c.is_ascii_digit()) {
                                mailbox.address.zip = last.to_string();
                            }
                        }
                        mailboxes.push(mailbox);
                    }
                }
            }
            
            // 防火墙策略：防止频繁请求被封IP
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }

        info!("Spider finished. Captured {} total mailboxes.", mailboxes.len());
        Ok(mailboxes)
    }
}
