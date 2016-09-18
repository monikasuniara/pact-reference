//! The `pact_matching` crate provides the core logic to performing matching on HTTP requests
//! and responses. It implements the V1 Pact specification (https://github.com/pact-foundation/pact-specification/tree/version-1).

#![warn(missing_docs)]

extern crate rustc_serialize;
#[macro_use] extern crate log;
#[macro_use] extern crate p_macro;
#[macro_use] extern crate maplit;
#[macro_use] extern crate lazy_static;
extern crate regex;
extern crate semver;
#[macro_use] extern crate itertools;
extern crate rand;
#[macro_use] extern crate hyper;

/// Simple macro to convert a string slice to a `String` struct.
#[macro_export]
macro_rules! s {
    ($e:expr) => ($e.to_string())
}

use std::collections::HashMap;
use std::iter::FromIterator;
use rustc_serialize::json::{Json, ToJson};

pub mod models;
mod json;

fn strip_whitespace<'a, T: FromIterator<&'a str>>(val: &'a String, split_by: &'a str) -> T {
    val.split(split_by).map(|v| v.trim().clone() ).collect()
}

static BODY_MATCHERS: [(&'static str, fn(expected: &String, actual: &String, config: DiffConfig, mismatches: &mut Vec<Mismatch>)); 1] = [
    ("application/json", json::match_json)
];

/// Enum that defines the different types of mismatches that can occur.
#[derive(Debug, Clone)]
pub enum Mismatch {
    /// Request Method mismatch
    MethodMismatch {
        /// Expected request method
        expected: String,
        /// Actual request method
        actual: String
    },
    /// Request Path mismatch
    PathMismatch {
        /// expected request path
        expected: String,
        /// actual request path
        actual: String
    },
    /// Response status mismatch
    StatusMismatch {
        /// expected response status
        expected: u16,
        /// actual response status
        actual: u16
    },
    /// Request query mismatch
    QueryMismatch {
        /// query parameter name
        parameter: String,
        /// expected value
        expected: String,
        /// actual value
        actual: String,
        /// description of the mismatch
        mismatch: String
    },
    /// Header mismatch
    HeaderMismatch {
        /// header key
        key: String,
        /// expected value
        expected: String,
        /// actual value
        actual: String,
        /// description of the mismatch
        mismatch: String
    },
    /// Mismatch in the content type of the body
    BodyTypeMismatch {
        /// expected content type of the body
        expected: String,
        /// actual content type of the body
        actual: String
    },
    /// Body element mismatch
    BodyMismatch {
        /// path expression to where the mismatch occured
        path: String,
        /// expected value
        expected: Option<String>,
        /// actual value
        actual: Option<String>,
        /// description of the mismatch
        mismatch: String
    }
}

impl Mismatch {
    /// Converts the mismatch to a `Json` struct.
    pub fn to_json(&self) -> Json {
        match self {
            &Mismatch::MethodMismatch { expected: ref e, actual: ref a } => {
                let map = btreemap!{
                    s!("type") => s!("MethodMismatch").to_json(),
                    s!("expected") => e.to_json(),
                    s!("actual") => a.to_json()
                };
                Json::Object(map)
            },
            &Mismatch::PathMismatch { expected: ref e, actual: ref a } => {
                let map = btreemap!{
                    s!("type") => s!("PathMismatch").to_json(),
                    s!("expected") => e.to_json(),
                    s!("actual") => a.to_json()
                };
                Json::Object(map)
            },
            &Mismatch::StatusMismatch { expected: ref e, actual: ref a } => {
                let map = btreemap!{
                    s!("type") => s!("StatusMismatch").to_json(),
                    s!("expected") => e.to_json(),
                    s!("actual") => a.to_json()
                };
                Json::Object(map)
            },
            &Mismatch::QueryMismatch { parameter: ref p, expected: ref e, actual: ref a, mismatch: ref m } => {
                let map = btreemap!{
                    s!("type") => s!("QueryMismatch").to_json(),
                    s!("parameter") => p.to_json(),
                    s!("expected") => e.to_json(),
                    s!("actual") => a.to_json(),
                    s!("mismatch") => m.to_json()
                };
                Json::Object(map)
            },
            &Mismatch::HeaderMismatch { key: ref k, expected: ref e, actual: ref a, mismatch: ref m } => {
                let map = btreemap!{
                    s!("type") => s!("HeaderMismatch").to_json(),
                    s!("key") => k.to_json(),
                    s!("expected") => e.to_json(),
                    s!("actual") => a.to_json(),
                    s!("mismatch") => m.to_json()
                };
                Json::Object(map)
            },
            &Mismatch::BodyTypeMismatch { expected: ref e, actual: ref a } => {
                let map = btreemap!{
                    s!("type") => s!("BodyTypeMismatch").to_json(),
                    s!("expected") => e.to_json(),
                    s!("actual") => a.to_json()
                };
                Json::Object(map)
            },
            &Mismatch::BodyMismatch { path: ref p, expected: ref e, actual: ref a, mismatch: ref m } => {
                let map = btreemap!{
                    s!("type") => s!("BodyMismatch").to_json(),
                    s!("path") => p.to_json(),
                    s!("expected") => match e {
                        &Some(ref v) => v.to_json(),
                        &None => Json::Null
                    },
                    s!("actual") => match a {
                        &Some(ref v) => v.to_json(),
                        &None => Json::Null
                    },
                    s!("mismatch") => m.to_json()
                };
                Json::Object(map)
            }
        }
    }

    /// Returns the type of the mismatch as a string
    pub fn mismatch_type(&self) -> String {
        match *self {
            Mismatch::MethodMismatch { .. } => s!("MethodMismatch"),
            Mismatch::PathMismatch { .. } => s!("PathMismatch"),
            Mismatch::StatusMismatch { .. } => s!("StatusMismatch"),
            Mismatch::QueryMismatch { .. } => s!("QueryMismatch"),
            Mismatch::HeaderMismatch { .. } => s!("HeaderMismatch"),
            Mismatch::BodyTypeMismatch { .. } => s!("BodyTypeMismatch"),
            Mismatch::BodyMismatch { .. } => s!("BodyMismatch")
        }
    }

    /// Returns a summary string for this mismatch
    pub fn summary(&self) -> String {
        match *self {
            Mismatch::MethodMismatch { expected: ref e, .. } => format!("is a {} request", e),
            Mismatch::PathMismatch { expected: ref e, .. } => format!("to path '{}'", e),
            Mismatch::StatusMismatch { expected: ref e, .. } => format!("has status code {}", e),
            Mismatch::QueryMismatch { ref parameter, expected: ref e, .. } => format!("includes parameter '{}' with value '{}'", parameter, e),
            Mismatch::HeaderMismatch { ref key, expected: ref e, .. } => format!("includes header '{}' with value '{}'", key, e),
            Mismatch::BodyTypeMismatch { .. } => s!("has a matching body"),
            Mismatch::BodyMismatch { .. } => s!("has a matching body")
        }
    }

    /// Returns a formated string for this mismatch
    pub fn description(&self) -> String {
        match *self {
            Mismatch::MethodMismatch { expected: ref e, actual: ref a } => format!("expected {} but was {}", e, a),
            Mismatch::PathMismatch { expected: ref e, actual: ref a } => format!("expected '{}' but was '{}'", e, a),
            Mismatch::StatusMismatch { expected: ref e, actual: ref a } => format!("expected {} but was {}", e, a),
            Mismatch::QueryMismatch { ref mismatch, .. } => mismatch.clone(),
            Mismatch::HeaderMismatch { ref mismatch, .. } => mismatch.clone(),
            Mismatch::BodyTypeMismatch {  expected: ref e, actual: ref a } => format!("expected a '{}' body but was '{}'", e, a),
            Mismatch::BodyMismatch { ref path, ref mismatch, .. } => format!("{} -> {}", path, mismatch)
        }
    }
}

impl PartialEq for Mismatch {
    fn eq(&self, other: &Mismatch) -> bool {
        match (self, other) {
            (&Mismatch::MethodMismatch{ expected: ref e1, actual: ref a1 },
                &Mismatch::MethodMismatch{ expected: ref e2, actual: ref a2 }) => {
                e1 == e2 && a1 == a2
            },
            (&Mismatch::PathMismatch{ expected: ref e1, actual: ref a1 },
                &Mismatch::PathMismatch{ expected: ref e2, actual: ref a2 }) => {
                e1 == e2 && a1 == a2
            },
            (&Mismatch::StatusMismatch{ expected: ref e1, actual: ref a1 },
                &Mismatch::StatusMismatch{ expected: ref e2, actual: ref a2 }) => {
                e1 == e2 && a1 == a2
            },
            (&Mismatch::BodyTypeMismatch{ expected: ref e1, actual: ref a1 },
                &Mismatch::BodyTypeMismatch{ expected: ref e2, actual: ref a2 }) => {
                e1 == e2 && a1 == a2
            },
            (&Mismatch::QueryMismatch{ parameter: ref p1, expected: ref e1, actual: ref a1, mismatch: _ },
                &Mismatch::QueryMismatch{ parameter: ref p2, expected: ref e2, actual: ref a2, mismatch: _ }) => {
                p1 == p2 && e1 == e2 && a1 == a2
            },
            (&Mismatch::HeaderMismatch{ key: ref p1, expected: ref e1, actual: ref a1, mismatch: _ },
                &Mismatch::HeaderMismatch{ key: ref p2, expected: ref e2, actual: ref a2, mismatch: _ }) => {
                p1 == p2 && e1 == e2 && a1 == a2
            },
            (&Mismatch::BodyMismatch{ path: ref p1, expected: ref e1, actual: ref a1, mismatch: _ },
                &Mismatch::BodyMismatch{ path: ref p2, expected: ref e2, actual: ref a2, mismatch: _ }) => {
                p1 == p2 && e1 == e2 && a1 == a2
            },
            (_, _) => false
        }
    }
}

/// Enum that defines the configuration options for performing a match.
pub enum DiffConfig {
    /// If unexpected keys are allowed and ignored during matching.
    AllowUnexpectedKeys,
    /// If unexpected keys cause a mismatch.
    NoUnexpectedKeys
}

/// Matches the actual text body to the expected one.
pub fn match_text(expected: &String, actual: &String, mismatches: &mut Vec<Mismatch>) {
    if expected != actual {
        mismatches.push(Mismatch::BodyMismatch { path: s!("/"), expected: Some(expected.clone()),
            actual: Some(actual.clone()),
            mismatch: format!("Expected text '{}' but received '{}'", expected, actual) });
    }
}

/// Matches the actual request method to the expected one.
pub fn match_method(expected: String, actual: String, mismatches: &mut Vec<Mismatch>) {
    if expected.to_lowercase() != actual.to_lowercase() {
        mismatches.push(Mismatch::MethodMismatch { expected: expected, actual: actual });
    }
}

/// Matches the actual request path to the expected one.
pub fn match_path(expected: String, actual: String, mismatches: &mut Vec<Mismatch>) {
    if expected != actual {
        mismatches.push(Mismatch::PathMismatch { expected: expected, actual: actual });
    }
}

fn compare_query_parameter_value(key: &String, expected: &String, actual: &String,
    mismatches: &mut Vec<Mismatch>) {
    if expected != actual {
        mismatches.push(Mismatch::QueryMismatch { parameter: key.clone(),
            expected: expected.clone(),
            actual: actual.clone(),
            mismatch: format!("Expected '{}' but received '{}' for query parameter '{}'",
            expected, actual, key) });
    }
}

fn compare_query_parameter_values(key: &String, expected: &Vec<String>, actual: &Vec<String>,
    mismatches: &mut Vec<Mismatch>) {
    for (index, val) in expected.iter().enumerate() {
        if index < actual.len() {
            compare_query_parameter_value(key, val, &actual[index], mismatches);
        } else {
            mismatches.push(Mismatch::QueryMismatch { parameter: key.clone(),
                expected: format!("{:?}", expected),
                actual: format!("{:?}", actual),
                mismatch: format!("Expected query parameter '{}' value '{}' but was missing", key, val) });
        }
    }
}

fn match_query_values(key: &String, expected: &Vec<String>, actual: &Vec<String>,
    mismatches: &mut Vec<Mismatch>) {
    if expected.is_empty() && !actual.is_empty() {
        mismatches.push(Mismatch::QueryMismatch { parameter: key.clone(),
            expected: format!("{:?}", expected),
            actual: format!("{:?}", actual),
            mismatch: format!("Expected an empty parameter list for '{}' but received {:?}", key, actual) });
    } else {
        if expected.len() != actual.len() {
            mismatches.push(Mismatch::QueryMismatch { parameter: key.clone(),
                expected: format!("{:?}", expected),
                actual: format!("{:?}", actual),
                mismatch: format!(
                    "Expected query parameter '{}' with {} value(s) but received {} value(s)",
                    key, expected.len(), actual.len()) });
        }
        compare_query_parameter_values(key, expected, actual, mismatches);
    }
}

fn match_query_maps(expected: HashMap<String, Vec<String>>, actual: HashMap<String, Vec<String>>,
    mismatches: &mut Vec<Mismatch>) {
    for (key, value) in &expected {
        match actual.get(key) {
            Some(actual_value) => match_query_values(key, value, actual_value, mismatches),
            None => mismatches.push(Mismatch::QueryMismatch { parameter: key.clone(),
                expected: format!("{:?}", value),
                actual: "".to_string(),
                mismatch: format!("Expected query parameter '{}' but was missing", key) })
        }
    }
    for (key, value) in &actual {
        match expected.get(key) {
            Some(_) => (),
            None => mismatches.push(Mismatch::QueryMismatch { parameter: key.clone(),
                expected: "".to_string(),
                actual: format!("{:?}", value),
                mismatch: format!("Unexpected query parameter '{}' received", key) })
        }
    }
}

/// Matches the actual query parameters to the expected ones.
pub fn match_query(expected: Option<HashMap<String, Vec<String>>>,
    actual: Option<HashMap<String, Vec<String>>>, mismatches: &mut Vec<Mismatch>) {
    match (actual, expected) {
        (Some(aqm), Some(eqm)) => match_query_maps(eqm, aqm, mismatches),
        (Some(aqm), None) => for (key, value) in &aqm {
            mismatches.push(Mismatch::QueryMismatch { parameter: key.clone(),
                expected: "".to_string(),
                actual: format!("{:?}", value),
                mismatch: format!("Unexpected query parameter '{}' received", key) });
        },
        (None, Some(eqm)) => for (key, value) in &eqm {
            mismatches.push(Mismatch::QueryMismatch { parameter: key.clone(),
                expected: format!("{:?}", value),
                actual: "".to_string(),
                mismatch: format!("Expected query parameter '{}' but was missing", key) });
        },
        (None, None) => (),
    };
}

fn parse_charset_parameters(parameters: &[&str]) -> HashMap<String, String> {
    parameters.iter().map(|v| v.split("=").map(|p| p.trim()).collect::<Vec<&str>>())
        .fold(HashMap::new(), |mut map, name_value| {
            map.insert(name_value[0].to_string(), name_value[1].to_string());
            map
        })
}

fn match_content_type(expected: &String, actual: &String, mismatches: &mut Vec<Mismatch>) {
    let expected_values: Vec<&str> = strip_whitespace(expected, ";");
    let actual_values: Vec<&str> = strip_whitespace(actual, ";");
    let expected_parameters = expected_values.as_slice().split_first().unwrap();
    let actual_parameters = actual_values.as_slice().split_first().unwrap();
    let header_mismatch = Mismatch::HeaderMismatch { key: "Content-Type".to_string(),
        expected: format!("{}", expected),
        actual: format!("{}", actual),
        mismatch: format!("Expected header 'Content-Type' to have value '{}' but was '{}'",
            expected, actual) };

    if expected_parameters.0 == actual_parameters.0 {
        let expected_parameter_map = parse_charset_parameters(expected_parameters.1);
        let actual_parameter_map = parse_charset_parameters(actual_parameters.1);
        for (k, v) in expected_parameter_map {
            if actual_parameter_map.contains_key(&k) {
                if v != *actual_parameter_map.get(&k).unwrap() {
                    mismatches.push(header_mismatch.clone());
                }
            } else {
                mismatches.push(header_mismatch.clone());
            }
        }
    } else {
        mismatches.push(header_mismatch.clone());
    }
}

fn match_header_value(key: &String, expected: &String, actual: &String, mismatches: &mut Vec<Mismatch>) {
    if key.to_lowercase() == "content-type" {
        match_content_type(expected, actual, mismatches);
    } else if strip_whitespace::<String>(expected, ",") != strip_whitespace::<String>(actual, ",") {
        mismatches.push(Mismatch::HeaderMismatch { key: key.clone(),
            expected: format!("{}", expected),
            actual: format!("{}", actual),
            mismatch: format!("Expected header '{}' to have value '{}' but was '{}'", key, expected, actual) });
    }
}

fn find_entry(map: &HashMap<String, String>, key: &String) -> Option<(String, String)> {
    match map.keys().find(|k| k.to_lowercase() == key.to_lowercase() ) {
        Some(k) => map.get(k).map(|v| (key.clone(), v.clone()) ),
        None => None
    }
}

fn match_header_maps(expected: HashMap<String, String>, actual: HashMap<String, String>,
    mismatches: &mut Vec<Mismatch>) {
    for (key, value) in &expected {
        match find_entry(&actual, key) {
            Some((_, actual_value)) => match_header_value(key, value, &actual_value, mismatches),
            None => mismatches.push(Mismatch::HeaderMismatch { key: key.clone(),
                expected: format!("{:?}", value),
                actual: "".to_string(),
                mismatch: format!("Expected header '{}' but was missing", key) })
        }
    }
}

/// Matches the actual headers to the expected ones.
pub fn match_headers(expected: Option<HashMap<String, String>>,
    actual: Option<HashMap<String, String>>, mismatches: &mut Vec<Mismatch>) {
    match (actual, expected) {
        (Some(aqm), Some(eqm)) => match_header_maps(eqm, aqm, mismatches),
        (Some(_), None) => (),
        (None, Some(eqm)) => for (key, value) in &eqm {
            mismatches.push(Mismatch::HeaderMismatch { key: key.clone(),
                expected: format!("{:?}", value),
                actual: "".to_string(),
                mismatch: format!("Expected header '{}' but was missing", key) });
        },
        (None, None) => (),
    };
}

fn compare_bodies(mimetype: String, expected: &String, actual: &String, config: DiffConfig,
    mismatches: &mut Vec<Mismatch>) {
    match BODY_MATCHERS.iter().find(|mt| *mt.0 == mimetype) {
        Some(ref match_fn) => match_fn.1(expected, actual, config, mismatches),
        None => match_text(expected, actual, mismatches)
    }
}

/// Matches the actual body to the expected one. This takes into account the content type of each.
pub fn match_body(expected: &models::HttpPart, actual: &models::HttpPart, config: DiffConfig,
    mismatches: &mut Vec<Mismatch>) {
    if expected.mimetype() == actual.mimetype() {
        match (expected.body(), actual.body()) {
            (&models::OptionalBody::Missing, _) => (),
            (&models::OptionalBody::Null, &models::OptionalBody::Present(ref b)) => {
                mismatches.push(Mismatch::BodyMismatch { expected: None, actual: Some(b.clone()),
                    mismatch: format!("Expected empty body but received '{}'", b.clone()),
                    path: s!("/")});
            },
            (&models::OptionalBody::Empty, &models::OptionalBody::Present(ref b)) => {
                mismatches.push(Mismatch::BodyMismatch { expected: None, actual: Some(b.clone()),
                    mismatch: format!("Expected empty body but received '{}'", b.clone()),
                    path: s!("/")});
            },
            (&models::OptionalBody::Null, _) => (),
            (&models::OptionalBody::Empty, _) => (),
            (e, &models::OptionalBody::Missing) => {
                mismatches.push(Mismatch::BodyMismatch { expected: Some(e.value()), actual: None,
                    mismatch: format!("Expected body '{}' but was missing", e.value()),
                    path: s!("/")});
            },
            (_, _) => {
                compare_bodies(expected.mimetype(), &expected.body().value(), &actual.body().value(),
                    config, mismatches);
            }
        }
    } else if expected.body().is_present() {
        mismatches.push(Mismatch::BodyTypeMismatch { expected: expected.mimetype(),
            actual: actual.mimetype() });
    }
}

/// Matches the expected and actual requests.
pub fn match_request(expected: models::Request, actual: models::Request) -> Vec<Mismatch> {
    let mut mismatches = vec![];

    info!("comparing to expected request: {:?}", expected);
    match_body(&expected, &actual, DiffConfig::NoUnexpectedKeys, &mut mismatches);
    match_method(expected.method, actual.method, &mut mismatches);
    match_path(expected.path, actual.path, &mut mismatches);
    match_query(expected.query, actual.query, &mut mismatches);
    match_headers(expected.headers, actual.headers, &mut mismatches);

    mismatches
}

/// Matches the actual response status to the expected one.
pub fn match_status(expected: u16, actual: u16, mismatches: &mut Vec<Mismatch>) {
    if expected != actual {
        mismatches.push(Mismatch::StatusMismatch { expected: expected, actual: actual });
    }
}

/// Matches the actual and expected responses.
pub fn match_response(expected: models::Response, actual: models::Response) -> Vec<Mismatch> {
    let mut mismatches = vec![];

    info!("comparing to expected response: {:?}", expected);
    match_body(&expected, &actual, DiffConfig::AllowUnexpectedKeys, &mut mismatches);
    match_status(expected.status, actual.status, &mut mismatches);
    match_headers(expected.headers, actual.headers, &mut mismatches);

    mismatches
}

#[cfg(test)]
#[macro_use(expect)]
extern crate expectest;
#[cfg(test)]
extern crate quickcheck;

#[cfg(test)]
mod tests;