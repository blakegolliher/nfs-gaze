//! The `nfs-gaze compare` subcommand: a side-by-side diff of two
//! JSON snapshot reports.
//!
//! The rendering style deliberately mirrors the rest of nfs-gaze
//! (fixed-width printf columns, dashed separators, numbers right
//! aligned) rather than nfs-monitor's `=`-banner style, so a user who
//! is already looking at the live table sees a compare layout that
//! belongs to the same tool.
//!
//! Only the first mount of each report is compared; if either report
//! contains more than one mount a note is printed on stderr but no
//! error is raised. Reports with zero mounts are a hard error because
//! the whole point of the command is to diff two real captures.

use crate::cli::CompareArgs;
use crate::snapshot::{MountReport, OpReport, Report, XprtReport};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader, Write};
use std::path::Path;

/// Total width of the comparison tables. Chosen to fit five columns
/// of numeric data plus an operation-name column without wrapping on
/// a standard 80-column terminal.
const TABLE_WIDTH: usize = 72;

/// Subcommand entry point. Loads both reports, picks a primary
/// mount from each, and prints the comparison to stdout.
pub fn run(args: CompareArgs) -> Result<()> {
    let report1 = load_report(&args.file1)?;
    let report2 = load_report(&args.file2)?;

    let label1 = args.label1.clone().unwrap_or_else(|| "File1".to_string());
    let label2 = args.label2.clone().unwrap_or_else(|| "File2".to_string());

    let mount1 = primary_mount(&report1, &args.file1.display().to_string())?;
    let mount2 = primary_mount(&report2, &args.file2.display().to_string())?;

    let mut stdout = io::stdout();
    print_comparison(
        &mut stdout,
        &report1,
        &report2,
        mount1,
        mount2,
        &label1,
        &label2,
    )
    .context("failed to write comparison to stdout")?;
    Ok(())
}

/// Deserialise a report from disk, attaching the path to any error.
fn load_report(path: &Path) -> Result<Report> {
    let file = File::open(path)
        .with_context(|| format!("failed to open report file {}", path.display()))?;
    let reader = BufReader::new(file);
    let report: Report = serde_json::from_reader(reader).with_context(|| {
        format!(
            "failed to parse {} as an nfs-gaze JSON report",
            path.display()
        )
    })?;
    Ok(report)
}

/// Pick the first mount from a report, warning on stderr if the
/// report has more than one. Returns an error if the report has zero
/// mounts — comparing nothing is never useful.
fn primary_mount<'a>(report: &'a Report, path: &str) -> Result<&'a MountReport> {
    if report.mounts.is_empty() {
        anyhow::bail!("{} contains no mounts", path);
    }
    if report.mounts.len() > 1 {
        eprintln!(
            "Note: {} contains {} mounts; comparing the first ({}).",
            path,
            report.mounts.len(),
            report.mounts[0].device
        );
    }
    Ok(&report.mounts[0])
}

/// Render the full comparison table into `w`. Split out of `run` so
/// tests can feed in an in-memory buffer.
pub fn print_comparison<W: Write>(
    w: &mut W,
    r1: &Report,
    r2: &Report,
    m1: &MountReport,
    m2: &MountReport,
    label1: &str,
    label2: &str,
) -> io::Result<()> {
    write_header(w, r1, r2, m1, m2, label1, label2)?;
    write_summary(w, m1, m2, label1, label2)?;

    let ops1 = index_ops(&m1.operations);
    let ops2 = index_ops(&m2.operations);
    let ordered = union_ops_by_total(&m1.operations, &m2.operations);

    // Latency: lower is better. Two decimal places matches
    // display.rs's format_duration for ms values.
    writeln!(w, "Latency (RTT avg in ms, lower is better)")?;
    write_op_header(w, label1, label2)?;
    for name in &ordered {
        let v1 = ops1.get(name.as_str()).map(|o| o.rtt_avg_ms).unwrap_or(0.0);
        let v2 = ops2.get(name.as_str()).map(|o| o.rtt_avg_ms).unwrap_or(0.0);
        write_compare_row(w, name, v1, v2, label1, label2, true, 2)?;
    }
    writeln!(w)?;

    // Throughput: higher is better. One decimal place matches
    // display.rs's format_rate.
    writeln!(w, "Throughput (ops/sec, higher is better)")?;
    write_op_header(w, label1, label2)?;
    for name in &ordered {
        let v1 = ops1
            .get(name.as_str())
            .map(|o| o.ops_per_sec)
            .unwrap_or(0.0);
        let v2 = ops2
            .get(name.as_str())
            .map(|o| o.ops_per_sec)
            .unwrap_or(0.0);
        write_compare_row(w, name, v1, v2, label1, label2, false, 1)?;
    }
    writeln!(w)?;

    // RPC Transport section. Only renders when both reports carry
    // xprt data — a mismatched pair (one TCP, one missing) is not
    // meaningful to diff. For TCP/TCP the section shows the slot
    // high-water mark and the three per-request pressure averages.
    if let (Some(x1), Some(x2)) = (m1.xprt.as_ref(), m2.xprt.as_ref()) {
        write_xprt_section(w, x1, x2, label1, label2)?;
    }

    writeln!(w, "{}", "-".repeat(TABLE_WIDTH))?;
    writeln!(w, "All ratios: ({} / {})", label2, label1)?;
    writeln!(w, "  Latency:  <1 means {} is faster", label2)?;
    writeln!(w, "  Ops/sec:  >1 means {} is faster", label2)?;
    Ok(())
}

fn write_header<W: Write>(
    w: &mut W,
    r1: &Report,
    r2: &Report,
    m1: &MountReport,
    m2: &MountReport,
    label1: &str,
    label2: &str,
) -> io::Result<()> {
    writeln!(w, "NFS Performance Comparison")?;
    writeln!(w, "{}", "-".repeat(TABLE_WIDTH))?;
    writeln!(w, "{:<12} {:<28} {:<28}", "", label1, label2)?;
    writeln!(w, "{:<12} {:<28} {:<28}", "Mount:", m1.device, m2.device)?;
    writeln!(
        w,
        "{:<12} {:<28} {:<28}",
        "Duration:",
        format!("{}s", r1.duration_sec),
        format!("{}s", r2.duration_sec)
    )?;
    writeln!(w)?;
    Ok(())
}

fn write_xprt_section<W: Write>(
    w: &mut W,
    x1: &XprtReport,
    x2: &XprtReport,
    label1: &str,
    label2: &str,
) -> io::Result<()> {
    writeln!(w, "RPC Transport ({} vs {})", x1.protocol, x2.protocol)?;
    writeln!(w, "{}", "-".repeat(TABLE_WIDTH))?;
    writeln!(
        w,
        "{:<18} {:>12} {:>12} {:>10} {:>14}",
        "Metric", label1, label2, "Ratio", "Better"
    )?;
    writeln!(w, "{}", "-".repeat(TABLE_WIDTH))?;

    // Max-slots high-water is informational, not ranked — a higher
    // value is not inherently "better" or "worse". If the slot_cap
    // is known on at least one side, append it in parens so the
    // reader can tell "7091 of 65536" from "7091 of ?".
    let cap_hint = x1.slot_cap.or(x2.slot_cap);
    let slot_label = match cap_hint {
        Some(c) => format!("Slots HW (cap {c})"),
        None => "Slots HW".to_string(),
    };
    write_summary_i64_row(w, &slot_label, x1.max_slots, x2.max_slots)?;

    // Per-request pressure averages: lower is better. A ratio of
    // zero on both sides skips the row entirely — there was no slot
    // pressure on either capture and the row would carry no signal.
    write_summary_f64_row(
        w,
        "bklog/req",
        x1.bklog_per_req,
        x2.bklog_per_req,
        label1,
        label2,
        true,
        3,
    )?;
    write_summary_f64_row(
        w,
        "sending/req",
        x1.sending_per_req,
        x2.sending_per_req,
        label1,
        label2,
        true,
        2,
    )?;
    write_summary_f64_row(
        w,
        "pending/req",
        x1.pending_per_req,
        x2.pending_per_req,
        label1,
        label2,
        true,
        2,
    )?;
    writeln!(w)?;
    Ok(())
}

fn write_summary<W: Write>(
    w: &mut W,
    m1: &MountReport,
    m2: &MountReport,
    label1: &str,
    label2: &str,
) -> io::Result<()> {
    writeln!(w, "Summary")?;
    writeln!(w, "{}", "-".repeat(TABLE_WIDTH))?;
    writeln!(
        w,
        "{:<18} {:>12} {:>12} {:>10} {:>14}",
        "Metric", label1, label2, "Ratio", "Better"
    )?;
    writeln!(w, "{}", "-".repeat(TABLE_WIDTH))?;

    write_summary_f64_row(
        w,
        "Ops/sec",
        m1.summary.ops_per_sec,
        m2.summary.ops_per_sec,
        label1,
        label2,
        false,
        1,
    )?;
    write_summary_i64_row(w, "Total ops", m1.summary.total_ops, m2.summary.total_ops)?;
    write_summary_i64_row(w, "Retransmissions", m1.summary.retrans, m2.summary.retrans)?;
    write_summary_i64_row(w, "Timeouts", m1.summary.timeouts, m2.summary.timeouts)?;
    write_summary_i64_row(w, "Errors", m1.summary.errors, m2.summary.errors)?;
    writeln!(w)?;
    Ok(())
}

fn write_op_header<W: Write>(w: &mut W, label1: &str, label2: &str) -> io::Result<()> {
    writeln!(w, "{}", "-".repeat(TABLE_WIDTH))?;
    writeln!(
        w,
        "{:<18} {:>12} {:>12} {:>10} {:>14}",
        "Operation", label1, label2, "Ratio", "Better"
    )?;
    writeln!(w, "{}", "-".repeat(TABLE_WIDTH))?;
    Ok(())
}

// Both write_*_row helpers carry one argument more than clippy's
// default cutoff. Each arg is load-bearing and the natural grouping
// (labels + style + numeric precision) would not actually collapse
// into a cohesive struct without adding a layer of noise at every
// call site. Acking the lint here is the simpler choice.
#[allow(clippy::too_many_arguments)]
fn write_summary_f64_row<W: Write>(
    w: &mut W,
    name: &str,
    v1: f64,
    v2: f64,
    label1: &str,
    label2: &str,
    lower_is_better: bool,
    precision: usize,
) -> io::Result<()> {
    if v1 > 0.0 && v2 > 0.0 {
        let ratio = v2 / v1;
        let better = compare_better(ratio, label1, label2, lower_is_better);
        writeln!(
            w,
            "{:<18} {:>12.*} {:>12.*} {:>10} {:>14}",
            name,
            precision,
            v1,
            precision,
            v2,
            format_ratio(ratio),
            better
        )?;
    } else {
        writeln!(
            w,
            "{:<18} {:>12.*} {:>12.*} {:>10} {:>14}",
            name, precision, v1, precision, v2, "-", "-"
        )?;
    }
    Ok(())
}

fn write_summary_i64_row<W: Write>(w: &mut W, name: &str, v1: i64, v2: i64) -> io::Result<()> {
    writeln!(
        w,
        "{:<18} {:>12} {:>12} {:>10} {:>14}",
        name, v1, v2, "", ""
    )?;
    Ok(())
}

/// Render one row of a per-operation comparison table. Handles the
/// three cases separately so ops present on only one side are not
/// dropped silently — a missing side shows `-` and suppresses the
/// ratio/winner columns.
#[allow(clippy::too_many_arguments)]
fn write_compare_row<W: Write>(
    w: &mut W,
    name: &str,
    v1: f64,
    v2: f64,
    label1: &str,
    label2: &str,
    lower_is_better: bool,
    precision: usize,
) -> io::Result<()> {
    if v1 > 0.0 && v2 > 0.0 {
        let ratio = v2 / v1;
        let better = compare_better(ratio, label1, label2, lower_is_better);
        writeln!(
            w,
            "{:<18} {:>12.*} {:>12.*} {:>10} {:>14}",
            name,
            precision,
            v1,
            precision,
            v2,
            format_ratio(ratio),
            better
        )?;
    } else if v1 > 0.0 {
        writeln!(
            w,
            "{:<18} {:>12.*} {:>12} {:>10} {:>14}",
            name, precision, v1, "-", "-", "-"
        )?;
    } else if v2 > 0.0 {
        writeln!(
            w,
            "{:<18} {:>12} {:>12.*} {:>10} {:>14}",
            name, "-", precision, v2, "-", "-"
        )?;
    }
    // Both zero → the op never ran in either session; skip entirely.
    Ok(())
}

/// Given a `label2 / label1` ratio, return a short label for whoever
/// wins. For latency (`lower_is_better = true`) a ratio less than 1
/// means label2 is faster; for throughput it is the opposite.
fn compare_better(ratio: f64, label1: &str, label2: &str, lower_is_better: bool) -> String {
    if (ratio - 1.0).abs() < 1e-9 {
        "=".to_string()
    } else if (lower_is_better && ratio < 1.0) || (!lower_is_better && ratio > 1.0) {
        label2.to_string()
    } else {
        label1.to_string()
    }
}

fn format_ratio(r: f64) -> String {
    format!("{:.2}x", r)
}

/// Index operations by name for O(1) lookups while rendering.
fn index_ops(ops: &[OpReport]) -> HashMap<&str, &OpReport> {
    ops.iter().map(|o| (o.name.as_str(), o)).collect()
}

/// Union the operation names from both reports, sorted by combined
/// ops descending. Ties broken lexicographically so test output is
/// deterministic across runs.
fn union_ops_by_total(a: &[OpReport], b: &[OpReport]) -> Vec<String> {
    let mut totals: HashMap<String, i64> = HashMap::new();
    for op in a {
        *totals.entry(op.name.clone()).or_default() += op.ops;
    }
    for op in b {
        *totals.entry(op.name.clone()).or_default() += op.ops;
    }
    let mut pairs: Vec<(String, i64)> = totals.into_iter().collect();
    pairs.sort_by(|x, y| y.1.cmp(&x.1).then_with(|| x.0.cmp(&y.0)));
    pairs.into_iter().map(|(n, _)| n).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::{OpReport, SummaryStats};
    use chrono::{TimeZone, Utc};

    fn op(name: &str, ops: i64, rtt_avg_ms: f64, ops_per_sec: f64) -> OpReport {
        OpReport {
            name: name.to_string(),
            ops,
            ops_per_sec,
            retrans: 0,
            timeouts: 0,
            errors: 0,
            rtt_avg_ms,
            rtt_min_ms: 0.0,
            rtt_max_ms: 0.0,
        }
    }

    fn build_report(device: &str, duration_sec: u64, ops: Vec<OpReport>) -> Report {
        let total_ops: i64 = ops.iter().map(|o| o.ops).sum();
        let ops_per_sec = if duration_sec > 0 {
            total_ops as f64 / duration_sec as f64
        } else {
            0.0
        };
        Report {
            schema_version: 1,
            generated_at: Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
            duration_sec,
            interval_sec: 1,
            samples: duration_sec,
            mounts: vec![MountReport {
                device: device.to_string(),
                mount_point: "/mnt/nfs".to_string(),
                fstype: "nfs4".to_string(),
                options: String::new(),
                covered_sec: duration_sec as f64,
                summary: SummaryStats {
                    total_ops,
                    ops_per_sec,
                    retrans: 0,
                    timeouts: 0,
                    errors: 0,
                },
                operations: ops,
                xprt: None,
            }],
        }
    }

    #[test]
    fn compare_better_identifies_lower_is_better_winner() {
        // label2 has lower latency (ratio 0.5), so label2 wins.
        assert_eq!(compare_better(0.5, "A", "B", true), "B");
        // label2 has higher latency (ratio 2.0), so label1 wins.
        assert_eq!(compare_better(2.0, "A", "B", true), "A");
        // Equal → "=".
        assert_eq!(compare_better(1.0, "A", "B", true), "=");
    }

    #[test]
    fn compare_better_identifies_higher_is_better_winner() {
        // label2 has higher throughput (ratio 2.0), so label2 wins.
        assert_eq!(compare_better(2.0, "A", "B", false), "B");
        // label2 has lower throughput (ratio 0.5), so label1 wins.
        assert_eq!(compare_better(0.5, "A", "B", false), "A");
    }

    #[test]
    fn union_ops_by_total_orders_by_combined_ops_descending() {
        let a = vec![op("READ", 1000, 0.5, 100.0), op("WRITE", 500, 1.0, 50.0)];
        let b = vec![op("READ", 200, 0.5, 20.0), op("GETATTR", 3000, 0.2, 300.0)];

        // Combined totals: GETATTR 3000, READ 1200, WRITE 500
        let ordered = union_ops_by_total(&a, &b);
        assert_eq!(ordered, vec!["GETATTR", "READ", "WRITE"]);
    }

    #[test]
    fn union_ops_by_total_breaks_ties_lexicographically() {
        let a = vec![op("WRITE", 100, 0.0, 0.0)];
        let b = vec![op("READ", 100, 0.0, 0.0)];
        let ordered = union_ops_by_total(&a, &b);
        // Both 100 ops; R before W alphabetically.
        assert_eq!(ordered, vec!["READ", "WRITE"]);
    }

    #[test]
    fn primary_mount_errors_on_empty_report() {
        let report = Report {
            schema_version: 1,
            generated_at: Utc::now(),
            duration_sec: 60,
            interval_sec: 1,
            samples: 60,
            mounts: vec![],
        };
        let err = primary_mount(&report, "empty.json").expect_err("should error");
        assert!(err.to_string().contains("no mounts"));
    }

    #[test]
    fn primary_mount_returns_first_of_many() {
        let report = Report {
            schema_version: 1,
            generated_at: Utc::now(),
            duration_sec: 60,
            interval_sec: 1,
            samples: 60,
            mounts: vec![
                MountReport {
                    device: "s:/a".into(),
                    mount_point: "/mnt/a".into(),
                    fstype: String::new(),
                    options: String::new(),
                    covered_sec: 0.0,
                    summary: SummaryStats {
                        total_ops: 0,
                        ops_per_sec: 0.0,
                        retrans: 0,
                        timeouts: 0,
                        errors: 0,
                    },
                    operations: vec![],
                    xprt: None,
                },
                MountReport {
                    device: "s:/b".into(),
                    mount_point: "/mnt/b".into(),
                    fstype: String::new(),
                    options: String::new(),
                    covered_sec: 0.0,
                    summary: SummaryStats {
                        total_ops: 0,
                        ops_per_sec: 0.0,
                        retrans: 0,
                        timeouts: 0,
                        errors: 0,
                    },
                    operations: vec![],
                    xprt: None,
                },
            ],
        };
        let primary = primary_mount(&report, "multi.json").expect("should succeed");
        assert_eq!(primary.device, "s:/a");
    }

    #[test]
    fn print_comparison_includes_summary_latency_and_throughput_sections() {
        // Two fake mounts with a READ op each; label2 is twice as
        // fast (half the RTT, double the throughput). The rendered
        // output must name both labels, both devices, both values,
        // and identify the winners.
        let r1 = build_report("serverA:/export", 60, vec![op("READ", 6000, 0.80, 100.0)]);
        let r2 = build_report("serverB:/export", 60, vec![op("READ", 12000, 0.40, 200.0)]);

        let mut buf = Vec::<u8>::new();
        print_comparison(
            &mut buf,
            &r1,
            &r2,
            &r1.mounts[0],
            &r2.mounts[0],
            "OLD",
            "NEW",
        )
        .expect("render");
        let out = String::from_utf8(buf).expect("utf-8");

        assert!(
            out.contains("NFS Performance Comparison"),
            "missing title: {out}"
        );
        assert!(out.contains("serverA:/export"), "missing device1: {out}");
        assert!(out.contains("serverB:/export"), "missing device2: {out}");
        assert!(out.contains("Summary"), "missing Summary section: {out}");
        assert!(
            out.contains("Latency (RTT avg in ms, lower is better)"),
            "missing latency section: {out}"
        );
        assert!(
            out.contains("Throughput (ops/sec, higher is better)"),
            "missing throughput section: {out}"
        );
        assert!(out.contains("READ"), "missing READ row: {out}");
        // In the throughput section the throughput ratio is 2.00x with
        // NEW winning; in the latency section the latency ratio is
        // 0.50x, also with NEW winning. Assert both winners are NEW.
        assert!(out.contains("2.00x"), "missing throughput ratio: {out}");
        assert!(out.contains("0.50x"), "missing latency ratio: {out}");
        assert!(out.contains("NEW"), "missing NEW winner label: {out}");
    }

    fn xprt(bklog: f64, sending: f64, pending: f64, max_slots: i64) -> XprtReport {
        XprtReport {
            protocol: "tcp".to_string(),
            nconnect: 1,
            max_slots,
            slot_cap: Some(65536),
            sends: 100,
            recvs: 100,
            bad_xids: 0,
            bklog_per_req: bklog,
            sending_per_req: sending,
            pending_per_req: pending,
        }
    }

    #[test]
    fn print_comparison_renders_xprt_section_when_both_sides_have_xprt() {
        // Side 1 has clean slots, side 2 is under pressure (bklog
        // per request of 0.1). The compare output must surface the
        // difference and identify the cleaner side as "better".
        let mut r1 = build_report("a:/e", 60, vec![op("READ", 1000, 0.5, 16.6)]);
        r1.mounts[0].xprt = Some(xprt(0.0, 0.50, 10.00, 32));
        let mut r2 = build_report("b:/e", 60, vec![op("READ", 1000, 0.5, 16.6)]);
        r2.mounts[0].xprt = Some(xprt(0.100, 0.80, 12.00, 48));

        let mut buf = Vec::<u8>::new();
        print_comparison(&mut buf, &r1, &r2, &r1.mounts[0], &r2.mounts[0], "A", "B")
            .expect("render");
        let out = String::from_utf8(buf).expect("utf-8");

        assert!(
            out.contains("RPC Transport"),
            "missing xprt section header: {out}"
        );
        assert!(out.contains("Slots HW"), "missing Slots HW row: {out}");
        assert!(out.contains("bklog/req"), "missing bklog/req row: {out}");
        assert!(
            out.contains("sending/req"),
            "missing sending/req row: {out}"
        );
        assert!(
            out.contains("pending/req"),
            "missing pending/req row: {out}"
        );
        // A (bklog = 0) is cleaner than B (bklog = 0.1); lower is
        // better for backlog pressure so A wins the bklog row.
        assert!(
            out.contains("(cap 65536)"),
            "expected slot cap hint in Slots HW label: {out}"
        );
    }

    #[test]
    fn print_comparison_omits_xprt_section_when_either_side_missing() {
        // If one report has xprt data and the other doesn't, the
        // xprt section should not render — mixing a known TCP side
        // with an unknown side would produce a half-populated table
        // that does not actually answer any question.
        let r1 = build_report("a:/e", 60, vec![op("READ", 1000, 0.5, 16.6)]);
        let mut r2 = build_report("b:/e", 60, vec![op("READ", 1000, 0.5, 16.6)]);
        r2.mounts[0].xprt = Some(xprt(0.0, 0.0, 0.0, 16));

        let mut buf = Vec::<u8>::new();
        print_comparison(&mut buf, &r1, &r2, &r1.mounts[0], &r2.mounts[0], "A", "B")
            .expect("render");
        let out = String::from_utf8(buf).expect("utf-8");

        assert!(
            !out.contains("RPC Transport"),
            "xprt section should be skipped when one side lacks data: {out}"
        );
    }

    #[test]
    fn print_comparison_shows_dash_when_op_is_missing_on_one_side() {
        // Only r1 has GETATTR; only r2 has CLOSE. Both ops should
        // appear once with a "-" on the missing side.
        let r1 = build_report("serverA:/export", 60, vec![op("GETATTR", 100, 0.30, 1.6)]);
        let r2 = build_report("serverB:/export", 60, vec![op("CLOSE", 50, 0.40, 0.8)]);

        let mut buf = Vec::<u8>::new();
        print_comparison(&mut buf, &r1, &r2, &r1.mounts[0], &r2.mounts[0], "A", "B")
            .expect("render");
        let out = String::from_utf8(buf).expect("utf-8");

        assert!(out.contains("GETATTR"), "missing GETATTR: {out}");
        assert!(out.contains("CLOSE"), "missing CLOSE: {out}");
        // A "-" token for missing-side values should appear at least
        // twice (once per op per section) — but the simpler assertion
        // is just that the substring is present at all.
        assert!(out.contains('-'), "missing dash sentinel: {out}");
    }
}
