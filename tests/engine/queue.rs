use rmp::engine::{filter_queue, mini_pick, next_index};
use rmp::proto::{MediaInfo, RepeatMode};

fn media(id: i64) -> MediaInfo {
    MediaInfo {
        id,
        uri: format!("/m/{id}.mp3"),
        kind: "offline".into(),
        title: Some(format!("Song {id}")),
        artist: None,
        duration: None,
        bitrate: None,
        source: None,
        tags: Vec::new(),
        favorite: false,
    }
}

fn queue(ids: &[i64]) -> Vec<i64> {
    filter_queue(
        &ids.iter().map(|i| media(*i)).collect::<Vec<_>>(),
        &[],
        None,
    )
}

#[test]
fn next_and_prev_walk_the_filtered_queue() {
    let q = queue(&[1, 2, 3]);
    let step =
        |cur: Option<i64>, dir: i32| next_index(&q, cur, dir, false, RepeatMode::Off, &mut |n| n);
    assert_eq!(step(Some(1), 1), Some(2));
    assert_eq!(step(Some(2), 1), Some(3));
    assert_eq!(step(Some(3), 1), None);
    assert_eq!(step(Some(3), -1), Some(2));
    assert_eq!(step(Some(2), -1), Some(1));
    assert_eq!(step(Some(1), -1), None);
    assert_eq!(step(None, 1), None, "unknown current does not advance");
}

#[test]
fn repeat_all_wraps_both_directions() {
    let q = queue(&[1, 2, 3, 4]);
    let step =
        |cur: Option<i64>, dir: i32| next_index(&q, cur, dir, false, RepeatMode::All, &mut |n| n);
    assert_eq!(step(Some(4), 1), Some(1));
    assert_eq!(step(Some(1), -1), Some(4));
    assert_eq!(step(None, -1), Some(4));
}

#[test]
fn repeat_one_never_advances() {
    let q = queue(&[1, 2, 3]);
    assert_eq!(
        next_index(&q, Some(2), 1, false, RepeatMode::One, &mut |n| n),
        Some(3),
        "repeat-one only affects track_ended, not next_index"
    );
}

#[test]
fn repeat_one_limits_single_item_queue() {
    let q = queue(&[9]);
    assert_eq!(
        next_index(&q, Some(9), 1, false, RepeatMode::Off, &mut |n| n),
        None
    );
    assert_eq!(
        next_index(&q, Some(9), 1, false, RepeatMode::All, &mut |n| n),
        Some(9)
    );
    assert_eq!(
        next_index(&q, Some(9), 1, true, RepeatMode::Off, &mut |n| n),
        Some(9)
    );
}

#[test]
fn removed_track_falls_back_to_last_main_position() {
    let q = queue(&[1, 2, 3]);
    assert_eq!(
        next_index(&q, Some(99), 1, false, RepeatMode::Off, &mut |n| n),
        None,
        "current not in queue has no next"
    );
}

#[test]
fn empty_queue_yields_none_everywhere() {
    let q: Vec<i64> = Vec::new();
    for dir in [1, -1] {
        for rep in [RepeatMode::Off, RepeatMode::All, RepeatMode::One] {
            assert_eq!(next_index(&q, None, dir, false, rep, &mut |n| n), None);
        }
    }
}

#[test]
fn mini_pick_respects_finished_removal() {
    let mut mini: Vec<i64> = vec![3, 1, 2];
    assert_eq!(mini_pick(&mut mini, Some(3)), Some(1));
    assert_eq!(mini_pick(&mut mini, Some(1)), Some(2));
    assert_eq!(mini_pick(&mut mini, Some(42)), Some(2), "unrelated finish");
    assert_eq!(mini_pick(&mut mini, Some(2)), None);
    assert_eq!(mini_pick(&mut mini, None), None);
}
