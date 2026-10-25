use camellia_remote_protocol::{protobuf::Message, rendezvous_proto::RendezvousMessage};
use std::{panic::AssertUnwindSafe, time::Instant};

const ITERATIONS: usize = 2_048;
const MAX_INPUT: usize = 8 * 1024;
const MAX_RUNTIME_MS: u128 = 10_000;

#[test]
fn malformed_rendezvous_corpus_is_panic_free_and_bounded() {
    let started = Instant::now();
    for index in 0..ITERATIONS {
        let mut state = 0x7265_6c61_792d_3230_u64.wrapping_add(index as u64);
        let len = (state as usize % MAX_INPUT).min(MAX_INPUT);
        let mut input = Vec::with_capacity(len);
        for _ in 0..len {
            state ^= state << 13;
            state ^= state >> 7;
            state = state.wrapping_mul(0x9e37_79b9_7f4a_7c15);
            input.push((state >> 32) as u8);
        }
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
            let _ = RendezvousMessage::parse_from_bytes(&input);
        }));
        assert!(result.is_ok(), "malformed message {index} panicked");
    }
    assert!(
        started.elapsed().as_millis() <= MAX_RUNTIME_MS,
        "message corpus exceeded runtime budget: {} ms",
        started.elapsed().as_millis()
    );
}
