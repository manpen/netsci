use ez_bitset::bitset::BitSet;
use rand::{seq::IteratorRandom, Rng};
use rand_distr::{Distribution, Hypergeometric};
use rayon::prelude::*;
use serde::Serialize;
use std::{collections::HashSet, hint::black_box, time::Instant};

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

fn gnm_epochs(rng: &mut impl Rng, size: usize, mut num_samples: usize) -> Vec<usize> {
    assert!(num_samples < size);

    let block_size = 1024.max(size >> 15);
    let mut buffer = vec![0u16; block_size];

    let mut result = Vec::with_capacity(num_samples);

    let mut block_begin = 0;
    let mut epoch = 1;
    while block_begin < size && num_samples != 0 {
        let block_end = (block_begin + block_size).min(size);
        let block_size = block_end - block_begin;

        let population = size - block_begin;

        let mut block_samples = if population == num_samples {
            num_samples
        } else {
            Hypergeometric::new(population as u64, num_samples as u64, block_size as u64)
                .unwrap_or_else(|e| panic!("{e} {population} {num_samples} {block_size}"))
                .sample(rng) as usize
        };
        num_samples -= block_samples;

        while block_samples != 0 {
            let idx = rng.gen_range(0..block_size);

            if buffer[idx] != epoch {
                buffer[idx] = epoch;
                result.push(block_begin + idx);
                block_samples -= 1;
            }
        }

        epoch += 1;
        block_begin = block_end;
    }

    result
}

#[derive(Clone, Copy)]
enum Algos {
    HashSet,
    Reservoir,
    BitSet,
    Epochs,
}

impl Algos {
    fn benchmark(self, series: &str, rng: &mut impl Rng, size: usize, samples: usize) -> Measure {
        match self {
            Algos::HashSet => Measure::bench(rng, gnm_hashset, "HashSet", series, size, samples),
            Algos::Reservoir => {
                Measure::bench(rng, gnm_reserviour, "Reservoir", series, size, samples)
            }
            Algos::BitSet => Measure::bench(rng, gnm_bitset, "Bitset", series, size, samples),
            Algos::Epochs => Measure::bench(rng, gnm_epochs, "Epochs", series, size, samples),
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
        (0..5)
            .into_par_iter()
            .flat_map(|_| (8..32).into_par_iter().map(|x| 1usize << x))
            .flat_map(|size| {
                [
                    Algos::HashSet,
                    Algos::Reservoir,
                    Algos::BitSet,
                    Algos::Epochs,
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
            })
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
