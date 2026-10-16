//! Before/after routing experiments (research/algorithms/03-routing-optimization.md).
//! `cargo run -p vl-sim --release --bin experiments`
//! Every cell is the mean of seeds 1..=5 on identical encounter traces.

use vl_routing::Algorithm;
use vl_sim::{Metrics, ReceiptLifetime, run_variant, run_with, standard_scenarios};

const SEEDS: [u64; 5] = [1, 2, 3, 4, 5];
const L8: Algorithm = Algorithm::SprayAndWait { initial: 8 };

fn mean(ms: &[Metrics], f: impl Fn(&Metrics) -> f64) -> f64 {
    let v: Vec<f64> = ms.iter().map(f).filter(|x| x.is_finite()).collect();
    if v.is_empty() {
        f64::NAN
    } else {
        v.iter().sum::<f64>() / v.len() as f64
    }
}

fn runs(s: &vl_sim::Scenario, r: ReceiptLifetime) -> Vec<Metrics> {
    SEEDS.iter().map(|seed| run_with(s, L8, *seed, r)).collect()
}

fn main() {
    println!("## E1 receipt lifetime (spray L=8)\n");
    println!(
        "| scenario | delivery | ack (receipt expires with data) | ack (fresh lifetime, production) | ack p50 min before → after |"
    );
    println!("|---|---|---|---|---|");
    let mut fresh_all = Vec::new();
    for s in standard_scenarios() {
        let before = runs(&s, ReceiptLifetime::WithData);
        let after = runs(&s, ReceiptLifetime::Fresh);
        println!(
            "| {} | {:.2} | {:.2} | {:.2} | {:.0} → {:.0} |",
            s.name,
            mean(&after, Metrics::delivery_ratio),
            mean(&before, Metrics::ack_ratio),
            mean(&after, Metrics::ack_ratio),
            mean(&before, |m| m.ack_latency_p50_min),
            mean(&after, |m| m.ack_latency_p50_min),
        );
        fresh_all.push((s.name, after));
    }
    println!("\n## E2 link bytes by message type (spray L=8, production receipts)\n");
    println!(
        "| scenario | SUMMARY share | VACCINE share | ENVELOPE share | other share | SUMMARY KiB per session |"
    );
    println!("|---|---|---|---|---|---|");
    for (name, ms) in &fresh_all {
        let share = |f: &dyn Fn(&Metrics) -> usize| {
            mean(ms, |m| {
                let total = m.bytes_summary + m.bytes_vaccine + m.bytes_envelope + m.bytes_other;
                f(m) as f64 / total.max(1) as f64
            })
        };
        println!(
            "| {name} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} |",
            share(&|m| m.bytes_summary),
            share(&|m| m.bytes_vaccine),
            share(&|m| m.bytes_envelope),
            share(&|m| m.bytes_other),
            mean(ms, |m| m.bytes_summary as f64
                / 1024.0
                / m.sessions.max(1) as f64),
        );
    }

    println!("\n## E3 why SUMMARY is large (spray L=8)\n");
    println!(
        "| scenario | entries per SUMMARY | redundant entries (peer already held or saw) | summaries from mid-session refresh |"
    );
    println!("|---|---|---|---|");
    for (name, ms) in &fresh_all {
        println!(
            "| {name} | {:.1} | {:.2} | {:.2} |",
            mean(ms, |m| m.summary_entries as f64 / m.summaries.max(1) as f64),
            mean(ms, |m| m.summary_entries_redundant as f64
                / m.summary_entries.max(1) as f64),
            mean(ms, |m| m.summaries_refresh as f64
                / m.summaries.max(1) as f64),
        );
    }

    println!("\n## E4 delta refresh (spray L=8): full refresh → delta refresh\n");
    println!("| scenario | delivery | ack | total link KiB | SUMMARY KiB | VACCINE KiB |");
    println!("|---|---|---|---|---|---|");
    for s in standard_scenarios() {
        let go = |delta: bool| -> Vec<Metrics> {
            SEEDS
                .iter()
                .map(|seed| {
                    run_variant(&s, L8, *seed, ReceiptLifetime::Fresh, |p| {
                        p.delta_refresh = delta
                    })
                })
                .collect()
        };
        let (a, b) = (go(false), go(true));
        let kib =
            |ms: &[Metrics], f: &dyn Fn(&Metrics) -> usize| mean(ms, |m| f(m) as f64 / 1024.0);
        let total =
            |m: &Metrics| m.bytes_summary + m.bytes_vaccine + m.bytes_envelope + m.bytes_other;
        println!(
            "| {} | {:.2} → {:.2} | {:.2} → {:.2} | {:.0} → {:.0} | {:.0} → {:.0} | {:.0} → {:.0} |",
            s.name,
            mean(&a, Metrics::delivery_ratio),
            mean(&b, Metrics::delivery_ratio),
            mean(&a, Metrics::ack_ratio),
            mean(&b, Metrics::ack_ratio),
            kib(&a, &total),
            kib(&b, &total),
            kib(&a, &|m| m.bytes_summary),
            kib(&b, &|m| m.bytes_summary),
            kib(&a, &|m| m.bytes_vaccine),
            kib(&b, &|m| m.bytes_vaccine),
        );
    }
}
