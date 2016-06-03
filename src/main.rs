// -*- coding:utf-8-unix -*-

#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate nickel;

extern crate regex;

use nickel::{Nickel, HttpRouter, Mountable, StaticFilesHandler};

use regex::Regex;

use std::cmp::Ordering;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io;
use std::path::Path;

const DOC_ROOT: &'static str = "public";

const LISTEN_ADDRESS: &'static str = "0.0.0.0";
const DEFAULT_PORT: &'static str = "6767";

const HOME_TEMPLATE: &'static str = "assets/home.mustache";

// Disable unreachable waring for `server.get("/", middleware! { ... })`
#[allow(unreachable_code)]

fn main() {
    let versions;
    match get_versions(DOC_ROOT) {
        Err(e) => {
            println!("An error occured while scanning the doc root directory. Exiting. \
                      Error: {}, Dir: {}",
                     e.to_string(),
                     DOC_ROOT);
            return;
        }
        Ok(vers) => {
            versions = vers;
        }
    }
    let menu_data = make_menu_data(&versions);

    let mut server = Nickel::new();

    // the home (menu) page
    server.get("/",
               middleware! {|_, response|
        return response.render(HOME_TEMPLATE, &menu_data); // need `return`
    });

    // set "public" folder as the document root
    server.mount("/", StaticFilesHandler::new(DOC_ROOT));

    // if there is no matching page in the previous mount, return "not found" message.
    // @TODO: Use a template with status 404
    server.mount("/",
                 middleware! { |req|
        let path = req.path_without_query().unwrap();
        format!("No static file with path '{}'!", path)
    });

    server.listen((LISTEN_ADDRESS, get_server_port()));
}


// NOTE: &str.to_string() vs &str.to_owned()
//
// In Rust 1.9 or newer, both methods should yeild the same performance,
// therefore to_string() will be more preferable than to_owned() for clarity.
// In older releases, you should replace these &str.to_string() calls with
// &str.to_owned() for better performance.


/// For Heroku deployment
fn get_server_port() -> u16 {
    env::var("PORT").unwrap_or(DEFAULT_PORT.to_string()).parse().unwrap()
}


/// Returns vec of version strings. e.g. vec!["1.10", "1.9", "1.6"]
fn get_versions(dir: &str) -> io::Result<Vec<String>> {
    let mut versions = try!(list_version_dirs(&Path::new(dir)));
    sort_versions(&mut versions);
    versions.reverse();
    Ok(versions.into_iter().map(|(_, _, ver)| ver).collect())
}

/// Returns vec of version tuples. e.g. vec![(1. 9, "1.9"), (1, 10, "1.10")]
fn list_version_dirs(dir: &Path) -> io::Result<Vec<(u32, u32, String)>> {
    lazy_static! {
        // NOTE: Assuming dir names are like 1.10, not 1.10.0
        static ref RE_SEM_VER: Regex = Regex::new(r".*/(\d+)\.(\d+)").unwrap();
    }

    let mut versions = Vec::new();

    if try!(fs::metadata(dir)).is_dir() {
        for entry in try!(fs::read_dir(dir)) {
            let entry = try!(entry);
            let metadata = try!(fs::metadata(entry.path()));
            if metadata.is_dir() {
                if let Some(path) = entry.path().to_str() {
                    if let Some(cap) = RE_SEM_VER.captures(path) {
                        let v1 = cap.at(1).unwrap().to_string();
                        let v2 = cap.at(2).unwrap().to_string();

                        // these `unwrap()` should not panic. we can trust regex `\d+`, can't we?
                        let ver =
                            (v1.parse().unwrap(), v2.parse().unwrap(), format!("{}.{}", v1, v2));
                        versions.push(ver);
                    }
                }
            }
        }
    }

    Ok(versions)
}

fn sort_versions(versions: &mut [(u32, u32, String)]) {
    versions.sort_by(|&(a0, a1, _), &(b0, b1, _)| {
        match a0.cmp(&b0) {
            Ordering::Equal => a1.cmp(&b1),
            other => other,
        }
    });
}

/// Returns a map for mustache template.
/// e.g. {"versions", [{"version", "1.10"}, {"version", "1.9"}, {"version", "1.6"}]}
fn make_menu_data(vers: &[String]) -> HashMap<String, Vec<HashMap<String, String>>> {
    let version_maps = vers.into_iter()
        .map(|ver| {
            let mut map = HashMap::new();
            map.insert("version".to_string(), ver.to_string());
            map
        })
        .collect();
    let mut menu_data = HashMap::new();
    menu_data.insert("versions".to_string(), version_maps);
    menu_data
}

#[cfg(test)]
mod tests {
    use super::{make_menu_data, sort_versions};

    #[test]
    fn sort_three_versions() {
        let mut versions =
            vec![(1, 10, "1.10".to_string()),
                 (1,  6, "1.6".to_string()),
                 (1,  9, "1.9".to_string())];
        let expectation =
            vec![(1,  6, "1.6".to_string()),
                 (1,  9, "1.9".to_string()),
                 (1, 10, "1.10".to_string())];

        sort_versions(&mut versions);
        assert_eq!(expectation, versions)
    }

    #[test]
    fn menu_data() {
        let versions = vec!["1.10".to_string(), "1.9".to_string(), "1.6".to_string()];
        let data = make_menu_data(&versions);

        let vers = data.get("versions").expect("versions should not be None.");
        for (expected, actual) in versions.iter().zip(vers.iter()) {
            assert_eq!(Some(expected), actual.get("version"));
        }
    }
}
