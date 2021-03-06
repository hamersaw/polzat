use std::collections::HashMap;
use regex::Regex;

use super::UrlType;

use curl::easy::Easy;

pub trait UrlValidator {
    fn is_valid(&mut self, url_type: &UrlType, url: &str) -> bool;
}

/*
 * RobotsValidator
 */
pub struct RobotsValidator {
    robots_cache: HashMap<String, Regex>,
}

impl RobotsValidator {
    pub fn new() -> RobotsValidator {
        RobotsValidator {
            robots_cache: HashMap::new(),
        }
    }
}

impl UrlValidator for RobotsValidator {
    fn is_valid(&mut self, url_type: &UrlType, url: &str) -> bool {
        if *url_type == UrlType::TorHiddenService {
            return true; //TODO url validate
        }

        //parse out domain and extension of url
        let url_clean = url.replace("https://", "").replace("http://", "");

        let index = url_clean.find("/");
        let domain = match index {
            Some(index) => &url_clean[..index],
            None => &url_clean,
        };

        let extension = match index {
            Some(index) => &url_clean[index..],
            None => "*",
        };

        //retrieve regex from map
        let decline_regex = self.robots_cache.entry(domain.to_owned()).or_insert_with(|| {
            //fetch and parse robots.txt for domain
            let mut buffer = vec!();
            let mut curl_handle = Easy::new();
            {
                //set curl handle parameters
                let _ = curl_handle.url(&format!("{}/robots.txt", domain)).expect("unable to set url RobotsValidator.is_valid()");
                let _ = curl_handle.follow_location(true).expect("unable to set follow_location RobotsValidator.is_valid()");

                //set transfer function
                let mut transfer = curl_handle.transfer();
                let _ = transfer.write_function(|data| {
                    buffer.extend_from_slice(data);
                    Ok(data.len())
                }).expect("unable to set transfer.write_function RobotsValidator.is_valid()");

                //submit curl request
                if transfer.perform().is_err() {
                    return Regex::new("a^").expect("unable to parse regex 'a^'")
                }
            }

            let response = String::from_utf8_lossy(&buffer).into_owned();
            let mut include_regex = false;
            let mut vec_disallow = Vec::new();
            for line in response.lines() {
                let mut fields = line.split_whitespace();
                let key = match fields.next() {
                    Some(key) => key,
                    None => continue,
                };

                if key == "User-agent:" {
                    if fields.next().expect("unable to parse user agent type") == "*" {
                        include_regex = true;
                    } else {
                        include_regex = false;
                    }
                } else if include_regex && key == "Disallow:" {
                    let url = match fields.next() {
                        Some(url) => url,
                        None => continue, //disallow without a url
                    };

                    vec_disallow.push(format!("{}.*", url.replace("?", "\\?").replace("*", ".*")).replace("**", "*"));
                }
            }

            match vec_disallow.len() {
                0 => Regex::new("a^").expect("unable to parse url 'a^'"), //match nothing
                _ => Regex::new(&vec_disallow.join("|")).expect("unable to parse regex correctly"),
            }
        });

        !decline_regex.is_match(extension)
    }
}
