use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

// Each worker sieves at most this many numbers at a time. Since the sieve uses
// one bit per number, this is about 8 MiB per worker.
const SEGMENT_NUMBER_COUNT: usize = 64_000_000;

// Convert a logical bit index into the byte that owns it and a mask for that
// specific bit. The same helper works for both the global base-prime sieve and
// each worker's local segment sieve.
fn bit_mask(i: usize) -> (usize, u8) {
    // Each u8 stores 8 prime/not-prime flags, so number i lives in byte i / 8.
    let byte_index = i / 8;

    // This creates a byte with only i's bit turned on.
    // Example: if i % 8 == 3, the mask is 0000_1000.
    let mask = 1u8 << (i % 8);

    (byte_index, mask)
}

// Read one packed boolean from the bitset.
fn get_bit(bits: &[u8], i: usize) -> bool {
    let (byte_index, mask) = bit_mask(i);

    // If this bit is set, i is still marked as prime.
    bits[byte_index] & mask != 0
}

// Mark one packed boolean as false.
fn clear_bit(bits: &mut [u8], i: usize) {
    let (byte_index, mask) = bit_mask(i);

    // !mask has every bit set except i's bit, so &= turns only that bit off.
    bits[byte_index] &= !mask;
}

// Count how many prime flags are still set. bit_count is the number of real
// flags; the backing Vec may have extra padding bits in its final byte.
fn count_set_bits(bits: &[u8], bit_count: usize) -> usize {
    let full_bytes = bit_count / 8;
    let remaining_bits = bit_count % 8;

    let mut count: usize = bits[..full_bytes]
        .iter()
        .map(|byte| byte.count_ones() as usize)
        .sum();

    if remaining_bits > 0 {
        // The final byte may include extra padding bits past n, so only count
        // the bits that represent actual numbers.
        let mask = (1u8 << remaining_bits) - 1;
        count += (bits[full_bytes] & mask).count_ones() as usize;
    }

    count
}

// Integer square root rounded down. This avoids relying on floating point for
// the final answer after using f64.sqrt() as a fast starting guess.
fn integer_sqrt(n: usize) -> usize {
    if n < 2 {
        return n;
    }

    let mut x = (n as f64).sqrt() as usize;

    while (x + 1) <= n / (x + 1) {
        x += 1;
    }

    while x > n / x {
        x -= 1;
    }

    x
}

// Build the list of primes needed to sieve every segment. To find primes up to
// n, it is enough for each segment to cross off multiples of primes up to
// sqrt(n).
fn small_primes_up_to(n: usize) -> Vec<usize> {
    if n < 2 {
        return Vec::new();
    }

    let bit_count = n + 1;

    // Round up to enough bytes to hold n + 1 bits.
    let byte_count = (bit_count + 7) / 8;

    // Start with every bit set to 1, meaning "assume prime until crossed off."
    let mut is_prime = vec![0xFFu8; byte_count];

    clear_bit(&mut is_prime, 0);
    clear_bit(&mut is_prime, 1);

    let mut p: usize = 2;

    // p <= n / p is the overflow-safe version of p * p <= n.
    while p <= n / p {
        if get_bit(&is_prime, p) {
            let mut i = p * p;
            while i <= n {
                clear_bit(&mut is_prime, i);
                i += p;
            }
        }
        p += 1;
    }

    (2..=n).filter(|&i| get_bit(&is_prime, i)).collect()
}

// Count primes in one inclusive range. This function is designed to be run by
// one thread, so each worker owns its own bitset and never writes shared memory.
fn count_primes_in_segment(low: usize, high: usize, base_primes: &[usize]) -> usize {
    let bit_count = high - low + 1;
    let byte_count = (bit_count + 7) / 8;
    let mut is_prime = vec![0xFFu8; byte_count];

    // In a segment, bit 0 represents `low`, not the number 0. Only segments
    // that actually contain 0 or 1 need those non-primes cleared by hand.
    if low == 0 {
        clear_bit(&mut is_prime, 0);

        if bit_count > 1 {
            clear_bit(&mut is_prime, 1);
        }
    } else if low == 1 {
        clear_bit(&mut is_prime, 0);
    }

    for &p in base_primes {
        let p_squared = p * p;

        // If p^2 is beyond this segment, larger base primes cannot cross off
        // anything here either.
        if p_squared > high {
            break;
        }

        // Start at the first multiple of p inside this segment, but never
        // before p^2. Smaller multiples were already crossed off by smaller
        // primes.
        let first_multiple = low.div_ceil(p) * p;
        let mut i = p_squared.max(first_multiple);

        while i <= high {
            // Convert the global number i into this segment's local bit index.
            clear_bit(&mut is_prime, i - low);
            i += p;
        }
    }

    count_set_bits(&is_prime, bit_count)
}

// Parallel segmented sieve of Eratosthenes.
fn sieve(n: usize) -> Duration {
    let start = Instant::now();

    if n < 2 {
        println!("Found 0 prime numbers up to {}.", n);
        println!();
        return start.elapsed();
    }

    let thread_count = thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(1);

    // These primes are small enough to compute once, then share read-only
    // across all worker threads.
    let base_primes = small_primes_up_to(integer_sqrt(n));

    let next_low = AtomicUsize::new(0);

    // thread::scope lets worker threads borrow base_primes without requiring
    // Arc or cloning. Rust guarantees all scoped threads finish before the
    // borrowed data can go away.
    let count: usize = thread::scope(|scope| {
        let mut handles = Vec::with_capacity(thread_count);

        for _ in 0..thread_count {
            let base_primes = &base_primes;
            let next_low = &next_low;

            // Each worker repeatedly claims a bounded segment. This keeps peak
            // memory proportional to thread_count, not to n.
            handles.push(scope.spawn(move || {
                let mut count = 0;

                loop {
                    let low = next_low.fetch_add(SEGMENT_NUMBER_COUNT, Ordering::Relaxed);

                    if low > n {
                        break;
                    }

                    let high = low.saturating_add(SEGMENT_NUMBER_COUNT - 1).min(n);
                    count += count_primes_in_segment(low, high, base_primes);
                }

                count
            }));
        }

        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .sum()
    });

    let elapsed = start.elapsed();

    println!("Found {} prime numbers up to {}.", count, n);
    println!("Used {} threads.", thread_count);
    println!();

    elapsed
}

fn main() {
    let n: usize = 10_000_000_000;

    let elapsed = sieve(n);

    println!("Ran in {:?}", elapsed)
}
