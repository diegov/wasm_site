extern crate unidecode;

use percent_encoding::percent_decode;
use unidecode::unidecode;
use url::Host;
use url::Url;

type R = Result<String, &'static str>;

pub fn abbreviate_max<'a>(
    url_string: &'a str,
    important_names: &[&str],
    desired_max_length: Option<usize>,
) -> R {
    abbreviate_impl(url_string, important_names, desired_max_length)
}

fn abbreviate_impl<'a>(
    url_string: &'a str,
    important_names: &[&str],
    desired_max_length: Option<usize>,
) -> R {
    let url_string = url_string;

    let mut url = match Url::parse(&url_string) {
        Ok(url) => url,
        Err(_) => return Err("Failed to parse URL"),
    };

    // From here on we will panic on errors, since nothing should fail once we have a valid URL

    // Remove www if present. We cannot call set_host inside these ifs because we've borrowed url already.
    // TODO: find idiomatic way to clean this up and remove new_host variable.
    let new_host = if let Some(Host::Domain(domain)) = url.host() {
        if let Some(first) = domain.split('.').next() {
            if first.to_lowercase() == "www" {
                Some(domain[4..].to_string())
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    if let Some(new_host) = new_host {
        url.set_host(Some(&new_host)).unwrap();
    }

    let new_path = if url.path().ends_with('/') {
        Some(url.path()[0..url.path().len() - 1].to_string())
    } else {
        None
    };

    if let Some(new_path) = new_path {
        url.set_path(&new_path);
    }

    let curr = url_to_string(&url);

    if let Some(length) = desired_max_length {
        if curr.len() > length {
            abbreviate_path(url, important_names)
        } else {
            Ok(curr)
        }
    } else {
        Ok(curr)
    }
}

fn url_to_string(url: &Url) -> String {
    let mut result = if let Some(Host::Domain(domain)) = url.host() {
        domain.to_owned()
    } else {
        "".to_string()
    };

    let path = percent_decode(url.path().as_bytes()).decode_utf8().unwrap();
    // Url adds a / for empty paths, so we'll remove them
    if !path.eq("/") {
        result.push_str(&path);
    }

    result
}

fn normalise<S: AsRef<str>>(text: S) -> String {
    unidecode(&text.as_ref().to_lowercase())
}

fn abbreviate_path(mut url: Url, names: &[&str]) -> R {
    let normalised_names: Vec<String> = names.iter().map(normalise).collect();

    let path = percent_decode(url.path().as_bytes())
        .decode_utf8()
        .map_err(|_| "Failed to decode UTF8")?;
    let components = split_path(path.as_ref());
    let mut remaining_names = normalised_names;

    let new_path = {
        let mut with_replacements: Vec<PathComponent> = vec![];

        let length = components.len();

        for (index, component) in components.into_iter().enumerate() {
            // We stop when we've found all the names we care about
            if !remaining_names.is_empty() {
                let result = if remaining_names.contains(component.get_normalised()) {
                    let pos = remaining_names
                        .iter()
                        .position(|el| *el == *component.get_normalised())
                        .unwrap();
                    remaining_names.remove(pos);
                    component
                } else {
                    replace_with_abbreviation(component)
                };

                // Remove trailing slash if present for last element
                if (index + 1) == length || remaining_names.is_empty() {
                    with_replacements.push(remove_trailing_slash(result));
                } else {
                    with_replacements.push(result);
                }
            }
        }

        rebuild_path(&with_replacements)
    };

    url.set_path(&new_path);

    Ok(url_to_string(&url))
}

fn remove_trailing_slash(component: PathComponent) -> PathComponent {
    if component.get_original().ends_with('/') {
        let normalised = component.get_normalised().to_string();
        let original = &component.get_original()[..component.get_original().len() - 1];
        PathComponent {
            original,
            normalised,
        }
    } else {
        component
    }
}

fn replace_with_abbreviation(component: PathComponent) -> PathComponent {
    // We keep the trailing /, paths look weird without them
    if component.get_original().ends_with('/') {
        if component.get_original().len() > 4 {
            return PathComponent {
                original: ".../",
                normalised: "".to_string(),
            };
        }
    } else if component.get_original().len() > 3 {
        return PathComponent {
            original: "...",
            normalised: "".to_string(),
        };
    }

    component
}

fn rebuild_path<'a, T: AsRef<PathComponent<'a>>>(path: &[T]) -> String {
    // Each path component will almost always be at least 3 caracters in length so we might as well preallocate that
    let mut result: String = String::with_capacity(path.len() * 3);
    for path_component in path {
        result.push_str(path_component.as_ref().get_original());
    }

    result
}

fn is_name_char(c: &char) -> bool {
    // This works surprisignly well over the entire unicode range
    c.is_alphabetic()
}

fn count_name_chars(v: &str) -> usize {
    for (index, c) in v.char_indices() {
        if !is_name_char(&c) {
            return index;
        }
    }

    v.len()
}

fn split_path(path: &str) -> Vec<PathComponent> {
    let mut result: Vec<PathComponent> = vec![];

    // The first attempt was using base-1 usizes, to leave 0 as null value, big mistake! char indices have gaps for
    // some unicode characters, so `index + 1` is not always a valid index to slice.
    // A signed int initialised to -1 is not a great option either, int to usize conversions are (correctly) treated
    // as fallible, requiring error handling, as are signed to unsigned conversions, so we stick to usize.
    let mut start_index: Option<usize> = None;
    let mut latest_alpha: Option<usize> = None;
    // Again, can't use any +1 -1 offsets due to gaps in indices, so we track the previous index. We can cheat
    // and init to 0 since we only compare for equality after the first iteration.
    let mut prev_index: usize = 0;
    // Again, cheat and save some Option syntactic awkwardness
    let mut latest_slash: usize = usize::MAX;

    for (index, c) in path.char_indices() {
        if let Some(start) = start_index {
            let (is_new_word, word_end) = match latest_alpha {
                Some(latest_alpha) => {
                    // There should be a gap of at least one other symbol for this to be considered the start of a new word, or we
                    // are just after a slash
                    let new_word = (is_name_char(&c) && latest_alpha != prev_index)
                        || latest_slash == prev_index;
                    let word_end = if latest_alpha >= start {
                        // We can't do latest_alpha + 1 due to index gaps
                        latest_alpha + count_name_chars(&path[latest_alpha..])
                    } else {
                        start
                    };
                    (new_word, word_end)
                }
                None => (true, start),
            };

            if is_new_word {
                // Collect what we have so far
                let original = &path[start..index];
                let normalised = normalise(&path[start..word_end]);
                result.push(PathComponent {
                    original,
                    normalised,
                });

                start_index = Some(index);
            }
        }

        if c == '/' {
            latest_slash = index;
        }

        if is_name_char(&c) {
            latest_alpha = Some(index);
        }

        if start_index.is_none() {
            start_index = Some(0);
        }

        prev_index = index;
    }

    if let Some(start) = start_index {
        let original = &path[start..];

        let word_end = match latest_alpha {
            Some(latest_alpha) => {
                if latest_alpha >= start {
                    latest_alpha + count_name_chars(&path[latest_alpha..])
                } else {
                    start
                }
            }
            None => start,
        };

        let normalised = normalise(&path[start..word_end]);

        result.push(PathComponent {
            original,
            normalised,
        });
    }

    result
}

struct PathComponent<'a> {
    original: &'a str,
    normalised: String,
}

impl<'a> Clone for PathComponent<'a> {
    fn clone(&self) -> Self {
        PathComponent {
            original: self.original,
            normalised: self.normalised.clone(),
        }
    }
}

impl<'a> PathComponent<'a> {
    fn get_original(&self) -> &'a str {
        self.original
    }

    fn get_normalised(&self) -> &String {
        &self.normalised
    }
}

impl<'a> AsRef<str> for PathComponent<'a> {
    fn as_ref(&self) -> &str {
        self.original
    }
}

// It seems this should be either automatic or something we could derive
impl<'a> AsRef<PathComponent<'a>> for PathComponent<'a> {
    fn as_ref(&self) -> &PathComponent<'a> {
        self
    }
}

#[cfg(test)]
mod tests {
    extern crate proptest;
    use super::*;
    use proptest::prelude::*;
    use proptest::test_runner::Config;

    fn abbreviate<'a>(url_string: &'a str, important_names: &[&str]) -> R {
        abbreviate_impl(url_string, important_names, None)
    }

    #[test]
    fn should_remove_scheme() {
        let url = "file:///test.csv";
        assert_eq!(abbreviate(url, &vec!()).unwrap(), "/test.csv");
    }

    #[test]
    fn should_remove_www() {
        let url = "ftp://www.testingdomain.co.uk";
        assert_eq!(abbreviate(url, &vec![]).unwrap(), "testingdomain.co.uk");
    }

    #[test]
    fn should_remove_trailing_slash() {
        let url = "http://testingdomain.co.uk/userstuff/";
        assert_eq!(
            abbreviate(url, &vec![]).unwrap(),
            "testingdomain.co.uk/userstuff"
        );
    }

    #[test]
    fn if_url_is_too_long_it_should_abbreviate_path_underscore() {
        let url = "http://www.test.co.uk/userstuff/john_doe/103914/";
        let names = vec!["John", "Doé"];
        assert_eq!(
            abbreviate_max(url, &names, Some(10)).unwrap(),
            "test.co.uk/.../john_doe"
        );
    }

    #[test]
    fn if_url_is_too_long_it_should_abbreviate_path_slash() {
        let url = "http://www.test.co.uk/userstuff/عاصم/Doe/39393939/";
        let names = vec!["عاصم", "Doe"];
        assert_eq!(
            abbreviate_max(url, &names, Some(10)).unwrap(),
            "test.co.uk/.../عاصم/Doe"
        );
    }

    #[test]
    fn if_url_is_too_long_it_should_abbreviate_path_name_only() {
        let url = "http://www.test.co.uk/userstuff/Christopher/103914/abcdef";
        let names = vec!["Christopher", "Alexander"];

        assert_eq!(
            abbreviate_max(url, &names, Some(12)).unwrap(),
            "test.co.uk/.../Christopher/.../..."
        );

        // TODO: This is what we really want here
        // assert_eq!(
        //     abbreviate_max(url, &names, Some(12)).unwrap(),
        //     "test.co.uk/.../Christopher"
        // );
    }

    #[test]
    fn if_url_is_too_long_it_should_abbreviate_path_dash() {
        let url = "http://www.test.co.uk/userstuff/robert-Smith/103914/abcdef";
        let names = vec!["Robert", "Smith"];
        assert_eq!(
            abbreviate_max(url, &names, Some(10)).unwrap(),
            "test.co.uk/.../robert-Smith"
        );
    }

    #[test]
    fn if_url_is_too_long_it_should_abbreviate_path_period() {
        let url = "http://www.test.co.uk/john.doe/is/great/abcdef";
        let names = vec!["John", "Doe"];
        assert_eq!(
            abbreviate_max(url, &names, Some(10)).unwrap(),
            "test.co.uk/john.doe"
        );
    }

    #[test]
    fn normalise_should_convert_to_lowercase() {
        assert_eq!(normalise(&"John"), normalise(&"john"));
    }

    #[test]
    fn normalise_should_remove_accents() {
        assert_eq!(normalise(&"Álvaro"), normalise(&"alvaro"));
    }

    #[test]
    fn path_split_should_normalise_single_component() {
        let path = "Hello";
        let parsed = split_path(&path);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].get_original(), "Hello");
        assert_eq!(parsed[0].get_normalised(), "hello");
    }

    #[test]
    fn path_split_should_normalise_components() {
        let path = "This/is/a-Path/Characters893";
        let parsed = split_path(&path);
        assert_eq!(parsed.len(), 5);

        assert_eq!(parsed[0].get_original(), "This/");
        assert_eq!(parsed[0].get_normalised(), "this");

        assert_eq!(parsed[1].get_original(), "is/");
        assert_eq!(parsed[1].get_normalised(), "is");

        assert_eq!(parsed[2].get_original(), "a-");
        assert_eq!(parsed[2].get_normalised(), "a");

        assert_eq!(parsed[3].get_original(), "Path/");
        assert_eq!(parsed[3].get_normalised(), "path");

        assert_eq!(parsed[4].get_original(), "Characters893");
        assert_eq!(parsed[4].get_normalised(), "characters");
    }

    #[test]
    fn path_split_should_normalise_components_starting_with_slash() {
        let path = "/userstuff/Christopher/103914/abcdef";
        let parsed = split_path(&path);
        assert_eq!(parsed.len(), 5);

        assert_eq!(parsed[0].get_original(), "/");
        assert_eq!(parsed[0].get_normalised(), "");

        assert_eq!(parsed[1].get_original(), "userstuff/");
        assert_eq!(parsed[1].get_normalised(), "userstuff");

        assert_eq!(parsed[2].get_original(), "Christopher/");
        assert_eq!(parsed[2].get_normalised(), "christopher");

        assert_eq!(parsed[3].get_original(), "103914/");
        assert_eq!(parsed[3].get_normalised(), "");

        assert_eq!(parsed[4].get_original(), "abcdef");
        assert_eq!(parsed[4].get_normalised(), "abcdef");
    }

    #[test]
    fn path_split_should_normalise_components_with_accents() {
        let path = "path/cómico-camión";
        let parsed = split_path(&path);
        assert_eq!(parsed.len(), 3);

        assert_eq!(parsed[0].get_original(), "path/");
        assert_eq!(parsed[0].get_normalised(), "path");

        assert_eq!(parsed[1].get_original(), "cómico-");
        assert_eq!(parsed[1].get_normalised(), "comico");

        assert_eq!(parsed[2].get_original(), "camión");
        assert_eq!(parsed[2].get_normalised(), "camion");
    }

    #[test]
    fn path_split_should_normalise_components_in_unicode() {
        let path = "path/عاصم-test";
        let parsed = split_path(&path);
        assert_eq!(parsed.len(), 3);

        assert_eq!(parsed[0].get_original(), "path/");
        assert_eq!(parsed[0].get_normalised(), "path");

        assert_eq!(parsed[1].get_original(), "عاصم-");
        // Not sure about this normalisation, it seems to lose too much information. But we have to trust
        // the unidecode crate...
        assert_eq!(parsed[1].get_normalised(), "`Sm");

        assert_eq!(parsed[2].get_original(), "test");
        assert_eq!(parsed[2].get_normalised(), "test");
    }

    #[test]
    fn path_split_should_normalise_components_single_car() {
        let path = "a/b#C-d_e^f";
        let parsed = split_path(&path);
        assert_eq!(parsed.len(), 6);

        assert_eq!(parsed[0].get_original(), "a/");
        assert_eq!(parsed[0].get_normalised(), "a");

        assert_eq!(parsed[1].get_original(), "b#");
        assert_eq!(parsed[1].get_normalised(), "b");

        assert_eq!(parsed[2].get_original(), "C-");
        assert_eq!(parsed[2].get_normalised(), "c");

        assert_eq!(parsed[3].get_original(), "d_");
        assert_eq!(parsed[3].get_normalised(), "d");

        assert_eq!(parsed[4].get_original(), "e^");
        assert_eq!(parsed[4].get_normalised(), "e");

        assert_eq!(parsed[5].get_original(), "f");
        assert_eq!(parsed[5].get_normalised(), "f");
    }

    proptest! {
        #![proptest_config(Config::with_cases(5000))]
        #[test]
        fn path_split_should_reconstruct_original(s in ".{0,50}") {
            //println!("{:?}", s);
            let parsed = split_path(&s);
            prop_assert_eq!(&rebuild_path(&parsed), &s);
        }
    }

    #[test]
    fn count_name_chars_should_count_starting_alpha_chars() {
        assert_eq!(count_name_chars("test_"), 4);
        assert_eq!(count_name_chars("test"), 4);
        assert_eq!(count_name_chars("?test_"), 0);
    }
}
