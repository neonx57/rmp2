use regex::Regex;
use vmp::engine::{filter_queue, matches, search_blob};
use vmp::proto::MediaInfo;

fn media(id: i64, title: &str, uri: &str, tags: &[&str]) -> MediaInfo {
    MediaInfo {
        id,
        uri: uri.to_string(),
        kind: "offline".into(),
        title: Some(title.to_string()),
        artist: Some("The Band".into()),
        duration: Some(100.0),
        bitrate: Some(320_000),
        source: None,
        tags: tags.iter().map(|s| s.to_string()).collect(),
        favorite: tags.contains(&"favorite"),
    }
}

#[test]
fn checked_tags_intersect_with_regex_filters() {
    let all = vec![
        media(1, "Rooftop", "/d/a.mp3", &["rock", "live"]),
        media(2, "Rooftop Live", "/d/b.mp3", &["rock", "live"]),
        media(3, "Sunset", "/d/c.mp3", &["jazz"]),
        media(4, "rooftop redux", "/d/e.mp3", &["rock"]),
    ];
    let tags = vec!["rock".to_string(), "live".to_string()];
    let q = filter_queue(&all, &tags, None);
    assert_eq!(q, vec![1, 2]);

    let re = Regex::new("sunset").unwrap();
    let q = filter_queue(&all, &tags, Some(&re));
    assert!(q.is_empty(), "no rock+live track matches sunset");

    let re = Regex::new("rooftop").unwrap();
    let q = filter_queue(&all, &tags, Some(&re));
    assert_eq!(q, vec![1, 2], "regex runs over the lowercased blob");

    let caps = Regex::new("ROOFTOP").unwrap();
    assert!(
        filter_queue(&all, &[], Some(&caps)).is_empty(),
        "search blob is precomposed lowercase: uppercase patterns never match"
    );
}

#[test]
fn matches_requires_all_checked_tags() {
    let m = media(5, "Any", "/x.mp3", &["chill", "rain"]);
    assert!(matches(&m, &[], None));
    assert!(matches(&m, &["chill".into()], None));
    assert!(matches(&m, &["chill".into(), "rain".into()], None));
    assert!(!matches(&m, &["chill".into(), "storm".into()], None));
    let re = Regex::new("rain").unwrap();
    assert!(matches(&m, &["chill".into()], Some(&re)));
    let re = Regex::new("nomatch").unwrap();
    assert!(!matches(&m, &[], Some(&re)));
}

#[test]
fn search_blob_covers_uri_and_tags() {
    let m = media(6, "/no-title", "/mp3s/Radio_Hit.mp3", &["Pop!"]);
    let blob = search_blob(&m);
    assert_eq!(blob, "/no-title the band /mp3s/radio_hit.mp3 pop!");
}

#[test]
fn filter_preserves_input_order() {
    let all = vec![
        media(30, "c", "/1.mp3", &["t"]),
        media(10, "a", "/2.mp3", &["t"]),
        media(20, "b", "/3.mp3", &[]),
    ];
    let q = filter_queue(&all, &[], None);
    assert_eq!(q, vec![30, 10, 20]);
}
