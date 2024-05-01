use ez_bitset::bitset::BitSet;
use rand::{prelude::*};
use rand_distr::{Binomial, Distribution, Hypergeometric};
use rayon::prelude::*;
use serde::Serialize;
use std::{collections::HashSet, hint::black_box, ops::Range, time::Instant};

#[derive(Serialize)]
struct Measure {
    size: usize,
    samples: usize,
    algo: String,
    seconds: f64,
    repeats: u64,
    series: String,
}

impl Measure {
    fn bench<R: Rng, F: Fn(&mut R, usize, usize) -> T, T>(
        rng: &mut R,
        func: F,
        algo: &str,
        series: &str,
        size: usize,
        samples: usize,
    ) -> Self {
        let (repeats, seconds) = {
            let mut repeats = 1;
            loop {
                let start = Instant::now();
                for _ in 0..repeats {
                    black_box(func(rng, size, samples));
                }
                let elapsed = start.elapsed().as_secs_f64();

                if elapsed > 0.1 {
                    break (repeats, elapsed / repeats as f64);
                }
                repeats *= 10;
            }
        };

        Self {
            size,
            samples,
            algo: String::from(algo),
            series: String::from(series),
            repeats,
            seconds,
        }
    }
}

fn gnm_hashset(rng: &mut impl Rng, size: usize, num_samples: usize) -> HashSet<usize> {
    assert!(num_samples < size);

    if num_samples > size / 2 {
        let complement = gnm_hashset(rng, size, size - num_samples);
        return (0..size).filter(|x| complement.contains(x)).collect();
    }

    let mut samples = HashSet::with_capacity(num_samples);

    while samples.len() < num_samples {
        let candidate = rng.gen_range(0..size);
        samples.insert(candidate);
    }

    samples
}

fn gnm_reserviour(rng: &mut impl Rng, size: usize, num_samples: usize) -> Vec<usize> {
    let mut res: Vec<usize> = (0..num_samples).collect();

    for x in num_samples..size {
        let idx = rng.gen_range(0..x);
        if idx < num_samples {
            res[idx] = x;
        }
    }

    res
}

fn gnm_bitset(rng: &mut impl Rng, size: usize, num_samples: usize) -> BitSet {
    assert!(num_samples < size);

    if num_samples > size / 2 {
        let mut result = gnm_bitset(rng, size, size - num_samples);
        result.not();
        return result;
    }

    let mut samples = BitSet::new(size);

    while samples.cardinality() < num_samples {
        let candidate = rng.gen_range(0..size);
        samples.set_bit(candidate);
    }

    samples
}

fn recurse(
    rng: &mut impl Rng,
    interval: Range<usize>,
    results: &mut [usize],
    bit_set_thresh: usize,
) {
    let size = interval.len();
    let num_samples = results.len();

    if size <= bit_set_thresh {
        let samples = gnm_bitset(rng, size, num_samples);
        for (r, x) in results.iter_mut().zip(samples.iter()) {
            *r = x;
        }

        return;
    }

    if num_samples * 100 < bit_set_thresh {
        let samples = gnm_hashset(rng, size, num_samples);
        for (r, x) in results.iter_mut().zip(samples.iter()) {
            *r = *x;
        }
        return;
    }

    if num_samples == 0 {
        return;
    }

    if num_samples == 1 {
        results[0] = rng.gen_range(interval);
        return;
    }

    let size_left = size / 2;
    let mid = interval.start + size_left;
    let samples_left = if size / num_samples > 10 {
        Binomial::new(size_left as u64, num_samples as f64 / size as f64)
            .unwrap()
            .sample(rng) as usize
    } else {
        Hypergeometric::new(size as u64, num_samples as u64, size_left as u64)
            .unwrap()
            .sample(rng) as usize
    }
    .min(num_samples);

    //    println!("{num_samples:>5} {samples_left:>5}");

    let (results_left, results_right) = results.split_at_mut(samples_left);
    recurse(rng, interval.start..mid, results_left, bit_set_thresh);
    recurse(rng, mid..interval.end, results_right, bit_set_thresh);
}

fn gnm_recursive_impl(
    rng: &mut impl Rng,
    size: usize,
    num_samples: usize,
    bit_set_thresh: usize,
) -> Vec<usize> {
    let mut results = vec![0; num_samples];
    recurse(rng, 0..size, &mut results, bit_set_thresh);
    //panic!();
    results
}

fn gnm_recursive(rng: &mut impl Rng, size: usize, num_samples: usize) -> Vec<usize> {
    gnm_recursive_impl(rng, size, num_samples, 0)
}

fn gnm_recursive_hybrid(rng: &mut impl Rng, size: usize, num_samples: usize) -> Vec<usize> {
    gnm_recursive_impl(rng, size, num_samples, 10_000)
}

#[derive(Clone, Copy)]
enum Algos {
    HashSet,
    Reservoir,
    BitSet,
    Recursive,
    RecursiveHybrid,
}

impl Algos {
    fn benchmark(self, series: &str, rng: &mut impl Rng, size: usize, samples: usize) -> Measure {
        match self {
            Algos::HashSet => Measure::bench(rng, gnm_hashset, "HashSet", series, size, samples),
            Algos::Reservoir => {
                Measure::bench(rng, gnm_reserviour, "Reservoir", series, size, samples)
            }
            Algos::BitSet => Measure::bench(rng, gnm_bitset, "Bitset", series, size, samples),
            Algos::Recursive => {
                Measure::bench(rng, gnm_recursive, "Recursive", series, size, samples)
            }
            Algos::RecursiveHybrid => Measure::bench(
                rng,
                gnm_recursive_hybrid,
                "Recursive Hybrid",
                series,
                size,
                samples,
            ),
        }
    }
}

fn main() {
    let size = 1 << 25;

    /*let measures: Vec<_> = if false {
        (0..5)
            .into_par_iter()
            .flat_map(|_| (0..size).into_par_iter().step_by(size / 20).skip(1))
            .flat_map(|k| {
                let mut rng = rand::thread_rng();
                [
                    Measure::bench(&mut rng, gnm_hashset, "HashSet", size, k),
                    Measure::bench(&mut rng, gnm_reserviour, "Reservoir", size, k),
                    Measure::bench(&mut rng, gnm_bitset, "Bitset", size, k),
                ]
            })
            .collect()
    } else */
    {
        let mut tasks : Vec<_> = (0..30)
            .into_par_iter()
            .flat_map(|_| (8..31).into_par_iter().map(|x| 1usize << x))
            .flat_map(|size| {
                [
                    Algos::HashSet,
                    Algos::Reservoir,
                    Algos::BitSet,
                    Algos::Recursive,
                    Algos::RecursiveHybrid,
                ]
                .into_par_iter()
                .map(move |a| (size, a))
            })
            .flat_map(|(size, algo)| {
                [
                    (algo, "k=10", size, 10usize),
                    (algo, "k=sqrt(N)", size, (size as f64).sqrt() as usize),
                    (algo, "k=N/4", size, size / 4),
                ]
                .into_par_iter()
            }).collect();

            tasks.shuffle(&mut rand::thread_rng());

            tasks.into_par_iter()
            .for_each(|(algo, series, size, samples)| {
                let mut rng = rand::thread_rng();
                println!(
                    "{}",
                    serde_json::to_string(&algo.benchmark(series, &mut rng, size, samples))
                        .unwrap()
                );
            });
    };

    //println!("{}", serde_json::to_string(&measures).unwrap());
}
