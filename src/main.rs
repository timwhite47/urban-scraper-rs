extern crate reqwest;
extern crate sitemap;
extern crate url;
#[macro_use] extern crate serde_derive;

extern crate serde;
extern crate serde_json;

use select::predicate::Predicate;
use sitemap::structs::{SiteMapEntry, UrlEntry};
use sitemap::reader::{SiteMapReader,SiteMapEntity};
use std::time::Duration;
use reqwest::Response;
use std::collections::{HashSet, HashMap};
use url::Url;
use rayon::prelude::*;
use select::document::Document;
use select::predicate::Class;
use select::predicate::Name;
use std::fs::File;

#[derive(Debug)]
pub enum SiteMapComponent {
    SiteMap(SiteMapEntry),
    Url(UrlEntry)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Entry {
    title: String,
    category: String,
    meaning: String,
    example: String,
    tags: Vec<String>,
    votes: HashMap<String, i32>
}

fn parse_sitemap_response(response: Response) -> HashSet<Url> {
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

fn fetch_and_parse_sitemap(client: &reqwest::Client, url: &str) -> HashSet<Url> {
    let response = client.get(url).send();
    match response {
        Ok(resp) => parse_sitemap_response(resp),
        Err(e) => {
            println!("{:?}", e);
            return HashSet::new()
        }
    }

}

fn parse_entry(response: Response) -> Vec<Entry> {
    let document = Document::from_read(response).unwrap();

    //  Get all definitions
    document.find(Class("def-panel")).into_iter().map(|definition| {
        // Title
        let header = definition
            .find(Class("def-header")).next().unwrap();

        let title = header
            .find(Class("word"))
            .next().unwrap().text();

        let category = header
            .find(Class("category"))
            .next().unwrap().text();

        // Meaning
        let meaning = definition
            .find(Class("meaning")).next().unwrap().text();

        // Example
        let example = definition
            .find(Class("example")).next().unwrap().text();

        // Votes
        let vote_node = definition.find(Class("thumbs")).next().unwrap();
        let up_votes: i32 = vote_node.find(Class("up")).next().unwrap().text().parse().unwrap();
        let down_votes: i32 = vote_node.find(Class("down")).next().unwrap().text().parse().unwrap();

        let mut votes = HashMap::new();
        votes.insert(String::from("up"), up_votes);
        votes.insert(String::from("down"), down_votes);

        // Tags
        let tags: Vec<String> = definition
            .find(Class("tags").descendant(Name("a")))
            .map(|tag| tag.text())
            .collect();

        return Entry {
            title,
            category,
            meaning,
            example,
            tags,
            votes
        }
    }).collect()
}

fn fetch_and_parse_entry(url: &str) -> Vec<Entry> {
    match reqwest::get(url) {
        Ok(resp) => parse_entry(resp),
        Err(e) => {
            println!("{}", e);

            return vec![]
        }
    }

}

fn main() {
    let base_url = "https://www.urbandictionary.com/sitemap.xml.gz";

    let client = reqwest::Client::builder()
        .gzip(true)
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();
    let sitemaps = &fetch_and_parse_sitemap(&client, &base_url);

    for (i, sitemap_url) in sitemaps.iter().enumerate() {
        println!("Fetching/parse: \t {}", sitemap_url);

        let entry_urls = fetch_and_parse_sitemap(&client, sitemap_url.as_str());
        let entries: Vec<Entry> = entry_urls.par_iter().flat_map(|entry_url| {
            fetch_and_parse_entry(entry_url.as_str())
        }).collect();

        // Serialize it to a JSON string.
        let fname = format!("data/data_{}.json", i);
        let f = File::create(fname).unwrap();
        serde_json::to_writer(&f, &entries).unwrap()
    };

}
