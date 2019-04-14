extern crate reqwest;
extern crate sitemap;
extern crate url;

use sitemap::structs::{SiteMapEntry, UrlEntry};
use sitemap::reader::{SiteMapReader,SiteMapEntity};
use std::time::Duration;
use reqwest::Response;
use std::collections::HashSet;
use url::Url;
use rayon::prelude::*;

#[derive(Debug)]
pub enum SiteMapComponent {
    SiteMap(SiteMapEntry),
    Url(UrlEntry)
}

fn parse_response(response: Response) -> HashSet<Url> {
    let mut components: HashSet<Url> = HashSet::new();

    let parser = SiteMapReader::new(response);

    for entity in parser {
        match entity {
            SiteMapEntity::Url(url_entry) => {
                let url = url_entry
                    .loc.get_url()
                    .expect("Could not parse url from `SiteMapEntity::Url`");

                components.insert(url);
            },
            SiteMapEntity::SiteMap(sitemap_entry) => {
                let url = sitemap_entry
                    .loc.get_url()
                    .expect("Could not parse url from `SiteMapEntity::SiteMap`");

                components.insert(url);
            },
            SiteMapEntity::Err(error) => {
                panic!("{}", error)
            },
        }
    }

    components
}

fn fetch_and_parse(client: &reqwest::Client, url: &str) -> HashSet<Url> {
    let response = client.get(url).send().unwrap();

    return parse_response(response);
}

fn main() {
    let base_url = "https://www.urbandictionary.com/sitemap.xml.gz";

    let client = reqwest::Client::builder()
        .gzip(true)
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();
    let sitemaps = &fetch_and_parse(&client, &base_url);

    sitemaps.par_iter().for_each(|sitemap_url| {
       for entry_url in &fetch_and_parse(&client, sitemap_url.as_str()) {
            print!("ENTRY \t {}", entry_url);
        }
    });
}
