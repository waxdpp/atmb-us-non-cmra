use color_eyre::eyre::{Context, OptionExt, Result};
use scraper::{Html, Selector};

#[derive(Debug)]
pub struct CountryPage {
    pub states: Vec<StateHtmlInfo>,
}

#[derive(Debug, Clone)]
pub struct StateHtmlInfo {
    pub sub_url: String,
    pub name: String,
}

impl StateHtmlInfo {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn url(&self) -> String {
        format!("https://anytimemailbox.com{}", self.sub_url)
    }
}

impl CountryPage {
    pub fn parse_html(html: &str) -> Result<Self> {
        let document = Html::parse_document(html);
        let selector = Selector::parse("a").unwrap();
        let mut states = Vec::new();

        for element in document.select(&selector) {
            if let Some(href) = element.value().attr("href") {
                if href.contains("/mailboxes/usa/") || href.contains("/l/usa/") {
                    let name = element.text().collect::<Vec<_>>().concat();
                    let name_trimmed = name.trim();
                    if !name_trimmed.is_empty() && name_trimmed.chars().all(|c| c.is_alphabetic() || c.is_whitespace()) {
                        states.push(StateHtmlInfo {
                            sub_url: href.to_string(),
                            name: name_trimmed.to_string(),
                        });
                    }
                }
            }
        }

        if states.is_empty() {
            color_eyre::eyre::bail!("No state found, page structure might be changed");
        }

        states.sort_by(|a, b| a.sub_url.cmp(&b.sub_url));
        states.dedup_by(|a, b| a.sub_url == b.sub_url);

        Ok(Self { states })
    }
}

#[derive(Debug)]
pub struct StatePage {
    pub locations: Vec<LocationHtmlInfo>,
}

impl StatePage {
    pub fn len(&self) -> usize {
        self.locations.len()
    }

    pub fn is_empty(&self) -> bool {
        self.locations.is_empty()
    }

    pub fn to_mailboxes(&self) -> Option<Vec<crate::atmb::model::Mailbox>> {
        let mut mailboxes = Vec::new();
        for loc in &self.locations {
            mailboxes.push(crate::atmb::model::Mailbox {
                name: loc.name.to_string(),
                id: loc.slug().to_string(),
                ..Default::default()
            });
        }
        if mailboxes.is_empty() {
            None
        } else {
            Some(mailboxes)
        }
    }

    pub fn parse_html(html: &str) -> Result<Self> {
        let document = Html::parse_document(html);
        let selector = Selector::parse("a").unwrap();
        let mut locations = Vec::new();

        for element in document.select(&selector) {
            if let Some(href) = element.value().attr("href") {
                if href.split('/').count() >= 5 && (href.contains("/mailboxes/usa/") || href.contains("/l/usa/")) {
                    let name = element.text().collect::<Vec<_>>().concat();
                    let name_trimmed = name.trim();
                    if !name_trimmed.is_empty() && !href.ends_with("/usa") && !href.ends_with("/usa/") {
                        locations.push(LocationHtmlInfo {
                            sub_url: href.to_string(),
                            name: name_trimmed.to_string(),
                        });
                    }
                }
            }
        }

        if locations.is_empty() {
            color_eyre::eyre::bail!("No locations found, state page structure might be changed");
        }

        locations.sort_by(|a, b| a.sub_url.cmp(&b.sub_url));
        locations.dedup_by(|a, b| a.sub_url == b.sub_url);

        Ok(Self { locations })
    }
}

#[derive(Debug, Clone)]
pub struct LocationHtmlInfo {
    pub sub_url: String,
    pub name: String,
}

impl LocationHtmlInfo {
    pub fn slug(&self) -> &str {
        self.sub_url.trim_end_matches('/').split('/').last().unwrap_or("")
    }
}

#[derive(Debug)]
pub struct LocationDetailPage {
    pub text: String,
}

impl LocationDetailPage {
    pub fn street(&self) -> String {
        self.text.lines().map(|s| s.trim()).filter(|s| !s.is_empty()).next().unwrap_or("").to_string()
    }

    pub fn parse_html(html: &str) -> Result<Self> {
        let document = Html::parse_document(html);
        let selector = Selector::parse("div, p, span, address").unwrap();
        let mut full_text = String::new();

        for element in document.select(&selector) {
            let text = element.text().collect::<Vec<_>>().concat().trim().to_string();
            if text.contains("USA") || text.split_whitespace().last().unwrap_or("").chars().all(|c| c.is_ascii_digit()) && text.split_whitespace().last().unwrap_or("").len() == 5 {
                full_text = text;
                break;
            }
        }

        Ok(Self { text: full_text })
    }
}
