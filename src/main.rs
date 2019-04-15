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
use select::node::Find;
use select::node::Node;

#[derive(Debug)]
pub enum SiteMapComponent {
    SiteMap(SiteMapEntry),
    Url(UrlEntry)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Entry {
    url: String,
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

fn parse_text(mut finder: Find<Class<&str>>) -> String {
    match finder.next() {
        Some(el) => el.text(),
        None => String::new(),
    }
}

fn parse_tags(definition: &Node) -> Vec<String> {
    definition
        .find(Class("tags").descendant(Name("a")))
        .map(|tag| tag.text())
        .collect()
}

fn parse_votes(definition: &Node) -> HashMap<String, i32> {
    let node = definition.find(Class("thumbs")).next();
    let mut votes = HashMap::new();

    match node {
        Some(vote_node) => {
            let up_node = vote_node.find(Class("up")).next();
            let down_node = vote_node.find(Class("down")).next();

            if up_node.is_some() && down_node.is_some() {
                let up = up_node.expect("Could not parse up votes");
                let down = down_node.expect("Could not parse down votes");

                let up_votes: i32 = up.text().parse().expect("Could not parse up votes");
                let down_votes: i32 = down.text().parse().expect("Could not parse down votes");

                votes.insert(String::from("up"), up_votes);
                votes.insert(String::from("down"), down_votes);
            }

        },
        None => {},
    }

    return votes
}

fn parse_entry(response: Response, url_str: &str) -> Vec<Option<Entry>> {
    let document = Document::from_read(response)
        .expect("Could not turn response into document");

    //  Get all definitions
    document.find(Class("def-panel")).into_iter().map(|definition| {
        let url = String::from(url_str);

        let header_el = definition
            .find(Class("def-header")).next();

        match header_el {
            Some(header) => {
                let title = parse_text(
                    header.find(Class("word"))
                );

               let category = parse_text(
                   header.find(Class("category"))
                );

                let meaning = parse_text(
                    definition.find(Class("meaning"))
                );

                let example = parse_text(
                    definition.find(Class("example"))
                );

                let tags: Vec<String> = parse_tags(&definition);

                let votes = parse_votes(&definition);

                let entry = Entry {
                    url,
                    title,
                    category,
                    meaning,
                    example,
                    tags,
                    votes
                };

                Some(entry)
            },
            None => None
        }


    }).collect()
}

fn fetch_and_parse_entry(url: &str) -> Vec<Option<Entry>> {
    match reqwest::get(url) {
        Ok(resp) => parse_entry(resp, url),
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
            let resp: Vec<Entry> = fetch_and_parse_entry(entry_url.as_str())
                    .into_iter()
                    .filter(|entry| entry.is_some())
                    .map(|entry| entry.expect("Could not get entry"))
                    .collect();

            resp

        }).collect();

        // Serialize it to a JSON string.
        let fname = format!("data/data_{}.json", i);
        let f = File::create(fname).unwrap();
        serde_json::to_writer(&f, &entries).unwrap()
    };

}
