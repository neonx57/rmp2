use regex::Regex;
use rmp::engine::{Lcg, filter_queue, next_index, search_blob};
use rmp::proto::{MediaInfo, RepeatMode};
use std::time::Instant;

fn synthetic_library(count: usize) -> Vec<MediaInfo> {
    (0..count)
        .map(|i| {
            let id = i as i64;
            MediaInfo {
                id,
                uri: format!("/music/album{}/track{:03}.flac", i % 40, i % 200),
                kind: if i % 7 == 0 {
                    "online".to_string()
                } else {
                    "offline".to_string()
                },
                title: Some(format!("Track {:03}", i % 250)),
                artist: Some(format!("Artist {}", i % 30)),
                duration: Some(120.0 + (i % 180) as f64),
                bitrate: Some(128_000 + (i % 4) as u64 * 16_000),
                source: None,
                tags: vec![format!("genre-{}", i % 6), format!("mood-{}", i % 4)],
                favorite: i % 9 == 0,
            }
        })
        .collect()
}

fn median_ns(samples: &[u128]) -> u128 {
    let mut v = samples.to_vec();
    v.sort_unstable();
    v[v.len() / 2]
}

fn time_samples<F: FnMut()>(label: &str, runs: usize, mut f: F) {
    let mut samples = Vec::new();
    for _ in 0..runs {
        let t0 = Instant::now();
        f();
        samples.push(t0.elapsed().as_nanos());
    }
    println!(
        "{label:<38} median {:>10} ns  ({} runs)",
        median_ns(&samples),
        runs
    );
}

fn main() {
    let lib = synthetic_library(5_000);
    let checked = vec!["genre-1".to_string(), "mood-1".to_string()];
    let re = Regex::new(r"track 0\d2").unwrap();

    println!("rmp engine micro-bench (std timing only)\n");

    time_samples("search_blob 5000 items", 20, || {
        let mut sink = 0usize;
        for m in &lib {
            sink += search_blob(m).len();
        }
        std::hint::black_box(sink);
    });

    time_samples("filter_queue 2 checked tags", 50, || {
        let q = filter_queue(&lib, &checked, None);
        std::hint::black_box(q.len());
    });

    time_samples("filter_queue 2 tags + regex", 50, || {
        let q = filter_queue(&lib, &checked, Some(&re));
        std::hint::black_box(q.len());
    });

    let q = filter_queue(&lib, &checked, None);
    let steps = 200_000;

    time_samples("next_index sequential x200k", 10, || {
        let mut cur = q[0];
        for _ in 0..steps {
            if let Some(n) = next_index(&q, Some(cur), 1, false, RepeatMode::Off, &mut |n| n) {
                cur = n;
            }
        }
        std::hint::black_box(cur);
    });

    time_samples("next_index repeat-all x200k", 10, || {
        let mut cur = q[0];
        for _ in 0..steps {
            if let Some(n) = next_index(&q, Some(cur), 1, false, RepeatMode::All, &mut |n| n) {
                cur = n;
            }
        }
        std::hint::black_box(cur);
    });

    let mut rng = Lcg::new();
    time_samples("next_index shuffle x200k", 10, || {
        let mut cur = q[0];
        for _ in 0..steps {
            if let Some(n) = next_index(&q, Some(cur), 1, true, RepeatMode::All, &mut |n| {
                rng.next(n)
            }) {
                cur = n;
            }
        }
        std::hint::black_box(cur);
    });

    time_samples("Lcg::next x200k", 10, || {
        let mut acc = 0usize;
        for _ in 0..steps {
            acc += rng.next(97);
        }
        std::hint::black_box(acc);
    });

    println!("\nqueue size after tag filter: {}", q.len());
}
