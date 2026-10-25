use std::collections::HashSet;
use vmp::engine::{Lcg, filter_queue, next_index};
use vmp::proto::{MediaInfo, RepeatMode};

fn media(id: i64, tags: &[&str]) -> MediaInfo {
    MediaInfo {
        id,
        uri: format!("/s/{id}.mp3"),
        kind: "offline".into(),
        title: Some(format!("Shuffle {id}")),
        artist: None,
        duration: None,
        bitrate: None,
        source: None,
        tags: tags.iter().map(|s| s.to_string()).collect(),
        favorite: false,
    }
}

#[test]
fn seeded_lcg_sequences_are_deterministic() {
    let mut a = Lcg::seeded(0x5eed);
    let mut b = Lcg::seeded(0x5eed);
    for n in [2usize, 3, 10, 50] {
        assert_eq!(a.next(n), b.next(n), "same seed must give same sequence");
    }
    let mut c = Lcg::seeded(0x5eed);
    assert_ne!(
        c.next(97),
        Lcg::seeded(0x5eed_u64.wrapping_add(1)).next(97),
        "different seeds diverge"
    );
    assert_eq!(Lcg::seeded(9).next(1), 0, "n <= 1 always yields 0");
}

#[test]
fn shuffle_never_returns_current_track() {
    let q: Vec<i64> = (0..25).map(|i| media(i, &[]).id).collect();
    let mut rng = Lcg::seeded(7);
    let cur = q[0];
    let mut seen: HashSet<i64> = HashSet::new();
    for _ in 0..1_000 {
        let next = next_index(&q, Some(cur), 1, true, RepeatMode::Off, &mut |n| {
            rng.next(n)
        })
        .expect("non-empty shuffle queue has a pick");
        assert_ne!(next, cur, "shuffle must not repeat the current track");
        assert!(q.contains(&next));
        seen.insert(next);
    }
    assert!(seen.len() > 1, "shuffle should vary picks over time");
}

#[test]
fn shuffle_reaches_every_track_of_a_small_queue() {
    let q: Vec<i64> = (1..=8).map(|i| media(i, &[]).id).collect();
    let mut rng = Lcg::seeded(11);
    let mut seen: HashSet<i64> = HashSet::new();
    for _ in 0..600 {
        let next =
            next_index(&q, None, 1, true, RepeatMode::All, &mut |n| rng.next(n)).expect("pick");
        seen.insert(next);
    }
    assert_eq!(seen.len(), 8, "every track is reachable by shuffle");
}

#[test]
fn shuffle_after_filter_only_picks_visible_ids() {
    let all: Vec<MediaInfo> = (0..40)
        .map(|i| {
            if i % 2 == 0 {
                media(i, &["rock"])
            } else {
                media(i, &["jazz"])
            }
        })
        .collect();
    let visible = filter_queue(&all, &["rock".to_string()], None);
    assert_eq!(visible.len(), 20);
    assert!(visible.iter().all(|id| id % 2 == 0));

    let mut rng = Lcg::seeded(23);
    for _ in 0..100 {
        let next = next_index(
            &visible,
            Some(visible[0]),
            1,
            true,
            RepeatMode::All,
            &mut |n| rng.next(n),
        )
        .expect("pick");
        assert!(visible.contains(&next));
    }
}
