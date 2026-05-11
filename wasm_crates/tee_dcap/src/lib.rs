use algokit::{avm_panic, read_global_bytes};
use dcap_qvl::quote::Quote;
use sha2::{Digest, Sha384};

type EventDigest = [u8; 48];

// IMR0 Events: 17
// IMR1 Events: 5
// IMR2 Events: 2
// IMR3 Events: 9

enum Rtmr {
    RTMR0 = 0,
    RTMR1 = 1,
    RTMR2 = 2,
    RTMR3 = 3,
}

impl Rtmr {
    fn range(&self) -> std::ops::Range<usize> {
        match self {
            Rtmr::RTMR0 => 0..17,
            Rtmr::RTMR1 => 17..22,
            Rtmr::RTMR2 => 22..24,
            Rtmr::RTMR3 => 24..33,
        }
    }
}

fn replay_rtmr(event_digests: &[EventDigest], rmtr: Rtmr) -> [u8; 48] {
    let mut mr = [0u8; 48];
    for digest in &event_digests[rmtr.range()] {
        let hasher = Sha384::default();
        mr = hasher
            .chain_update(mr)
            .chain_update(digest)
            .finalize()
            .into();
    }
    mr
}

#[derive(Debug, PartialEq, Eq)]
struct RMTRValues {
    rtmr0: [u8; 48],
    rtmr1: [u8; 48],
    rtmr2: [u8; 48],
    rtmr3: [u8; 48],
}

impl RMTRValues {
    fn from_event_digests(event_digests: &[EventDigest]) -> Self {
        Self {
            rtmr0: replay_rtmr(event_digests, Rtmr::RTMR0),
            rtmr1: replay_rtmr(event_digests, Rtmr::RTMR1),
            rtmr2: replay_rtmr(event_digests, Rtmr::RTMR2),
            rtmr3: replay_rtmr(event_digests, Rtmr::RTMR3),
        }
    }
}

const DSTACK_RUNTIME_EVENT_TYPE: u32 = 0x08000001;

pub enum Event {
    ComposeHash,
    AppId,
}

impl Event {
    fn name(&self) -> &'static str {
        match self {
            Event::ComposeHash => "compose-hash",
            Event::AppId => "app-id",
        }
    }

    fn calculate_digest(&self, payload: &[u8]) -> [u8; 48] {
        let mut hasher = Sha384::default();
        hasher.update(DSTACK_RUNTIME_EVENT_TYPE.to_le_bytes());
        hasher.update(b":");
        hasher.update(self.name().as_bytes());
        hasher.update(b":");
        hasher.update(payload);
        hasher.finalize().into()
    }

    fn validate_digest(&self, payload: &[u8], digest: &[u8; 48]) {
        let calculated_digest = self.calculate_digest(payload);
        assert_eq!(
            &calculated_digest,
            digest,
            "{} digest mismatch: quoted {} does not match replayed {}",
            self.name(),
            self.name(),
            self.name()
        );
    }
}

const EVENT_COUNT: i32 = 10;

#[unsafe(export_name = "program")]
pub extern "C" fn program() -> u64 {
    let mut quote_bytes = [0u8; 128];
    if read_global_bytes(0, b"quote", &mut quote_bytes).is_err() {
        avm_panic();
    }

    let mut event_digests = Vec::new();

    for _ in 0..EVENT_COUNT {
        let mut buf = [0u8; 48];
        if read_global_bytes(0, b"digests", &mut buf).is_err() {
            avm_panic();
        }
        event_digests.push(buf);
    }

    let mut compose_hash = [0u8; 256];
    if read_global_bytes(0, b"compose_hash", &mut compose_hash).is_err() {
        avm_panic();
    }

    let mut app_id = [0u8; 256];
    if read_global_bytes(0, b"phala_app_id", &mut app_id).is_err() {
        avm_panic();
    }

    // NOTE: Collateral verification is skipped in this guest code since we're not using a real TDX
    // server yet
    //
    // let collateral_bytes = sp1_zkvm::io::read::<Vec<u8>>();
    // let time = sp1_zkvm::io::read::<u64>();
    //
    // let collateral: QuoteCollateralV3 =
    //     borsh::from_slice(collateral_bytes.as_slice()).expect("failed to deserialize collateral");
    //
    // verify(&quote_bytes, &collateral, time).expect("failed to verify quote");

    let rtmr3_digests = &event_digests[Rtmr::RTMR3.range()];
    let app_id_digest = rtmr3_digests.get(1).expect("should have app id digest");
    Event::AppId.validate_digest(&app_id, app_id_digest);

    let compose_hash_digest = rtmr3_digests
        .get(2)
        .expect("should have compose hash digest");
    Event::ComposeHash.validate_digest(&compose_hash, compose_hash_digest);

    let quote: Quote = Quote::parse(&quote_bytes).expect("failed to parse quote");

    let report = quote.report.as_td10().expect("failed to get td10 report");
    let replayed_rtmr_values = RMTRValues::from_event_digests(&event_digests);
    let quoted_rtmr_values = RMTRValues {
        rtmr0: report.rt_mr0,
        rtmr1: report.rt_mr1,
        rtmr2: report.rt_mr2,
        rtmr3: report.rt_mr3,
    };

    assert_eq!(
        replayed_rtmr_values, quoted_rtmr_values,
        "RTMR3 mismatch: quoted RTMR3 does not match replayed RTMR3"
    );

    assert_eq!(
        report.report_data[32..],
        [0u8; 32],
        "Report data is longer than 32 bytes"
    );

    1
}
