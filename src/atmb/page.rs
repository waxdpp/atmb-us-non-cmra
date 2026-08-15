use anyhow::{bail, Context, Result};
use scraper::{Html, Selector};

// 保留原作者定义的、其他文件依赖的三个核心核心结构体
#[derive(Debug)]
pub struct CountryPage<'a> {
    pub states: Vec<StateHtmlInfo<'a>>,
}

#[derive(Debug)]
pub struct StateHtmlInfo<'a> {
    pub sub_url: &'a str,
    pub name: &'a str,
}

#[derive(Debug)]
pub struct StatePage<'a> {
    pub locations: Vec<LocationHtmlInfo<'a>>,
}

#[derive(Debug)]
pub struct LocationHtmlInfo<'a> {
    pub sub_url: &'a str,
    pub name: &'a str,
}

#[derive(Debug)]
pub struct LocationDetailPage {
    pub text: String,
}

impl LocationDetailPage {
    pub fn street(&self) -> String {
        // 简单提取首行作为街道，防止 mod.rs 报错
        self.text.lines().next().unwrap_or("").trim().to_string()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct State {
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Location {
    pub name: String,
    pub slug: String,
}

/// 1. 解析国家页面下的所有州 (兼容2026最新ATMB版网页)
pub fn parse_states(html: &str) -> Result<Vec<State>> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("a").unwrap();
    let mut states = Vec::new();

    for element in document.select(&selector) {
        if let Some(href) = element.value().attr("href") {
            if href.contains("/mailboxes/usa/") || href.contains("/l/usa/") {
                let slug = href.trim_end_matches('/').split('/').last().unwrap_or("").to_string();
                let name = element.text().collect::<Vec<_>>().concat().trim().to_string();
                if !slug.is_empty() && !name.is_empty() && name.chars().all(|c| c.is_alphabetic() || c.is_whitespace()) {
                    states.push(State { name, slug });
                }
            }
        }
    }

    if states.is_empty() {
        bail!("No state found, page structure might be changed");
    }

    states.sort_by(|a, b| a.slug.cmp(&b.slug));
    states.dedup_by(|a, b| a.slug == b.slug);

    Ok(states)
}

/// 2. 解析某个州页面下的所有具体网点 (Locations)
pub fn parse_locations(html: &str) -> Result<Vec<Location>> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("a").unwrap();
    let mut locations = Vec::new();

    for element in document.select(&selector) {
        if let Some(href) = element.value().attr("href") {
            if href.split('/').count() >= 5 && (href.contains("/mailboxes/usa/") || href.contains("/l/usa/")) {
                let slug = href.trim_end_matches('/').split('/').last().unwrap_or("").to_string();
                let name = element.text().collect::<Vec<_>>().concat().trim().to_string();
                if !slug.is_empty() && !name.is_empty() {
                    locations.push(Location { name, slug });
                }
            }
        }
    }

    if locations.is_empty() {
        bail!("No locations found, state page structure might be changed");
    }

    locations.sort_by(|a, b| a.slug.cmp(&b.slug));
    locations.dedup_by(|a, b| a.slug == b.slug);

    Ok(locations)
}

/// 3. 解析具体网点页面的物理地址数据
pub fn parse_addresses(html: &str) -> Result<Vec<String>> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("div, p, span, address").unwrap();
    let mut addresses = Vec::new();

    for element in document.select(&selector) {
        let text = element.text().collect::<Vec<_>>().concat().trim().to_string();
        if text.contains("USA") || text.split_whitespace().last().unwrap_or("").chars().all(|c| c.is_ascii_digit()) && text.split_whitespace().last().unwrap_or("").len() == 5 {
            if text.len() > 10 && text.len() < 150 && !addresses.contains(&text) {
                addresses.push(text);
            }
        }
    }

    Ok(addresses)
}
