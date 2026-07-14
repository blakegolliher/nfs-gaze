//! Golden-corpus tests for the mountstats parser.
//!
//! `tests/fixtures/mountstats/` holds realistic `/proc/self/mountstats`
//! captures — including a real nconnect=16 block captured from a
//! production-style host, an NFSv4.1 mount with an `impl_id:` line, an
//! fscache mount with the legacy `fsc:` stats line (kernels <= 5.17,
//! e.g. RHEL/Rocky 9), a UDP transport, and a deliberately hostile
//! file. Every fixture must parse without error: the parser's contract
//! is that only I/O failures abort a parse, never file content.

use nfs_gaze::parser::parse_mountstats_reader;
use nfs_gaze::NFSMount;
use std::collections::HashMap;
use std::io::Cursor;
use std::path::PathBuf;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("mountstats")
}

fn load_fixture(name: &str) -> String {
    let path = fixture_dir().join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {}: {e}", path.display()))
}

fn parse_fixture(name: &str) -> HashMap<String, NFSMount> {
    let content = load_fixture(name);
    parse_mountstats_reader(Cursor::new(content))
        .unwrap_or_else(|e| panic!("fixture {name} must parse without error, got: {e}"))
}

#[test]
fn corpus_nfs3_nconnect16_parses_fully() {
    // Sanitized real capture: NFSv3, nconnect=16 (sixteen xprt lines),
    // O_DIRECT-heavy workload. Values below were cross-checked against
    // the raw file with awk at capture time.
    let mounts = parse_fixture("nfs3_nconnect16.txt");
    assert_eq!(mounts.len(), 1);

    let mount = &mounts["/mnt/data"];
    assert_eq!(mount.device, "nfs-server01.example.com:/export/data");
    assert_eq!(mount.server, "nfs-server01.example.com");
    assert_eq!(mount.export, "/export/data");

    let getattr = &mount.operations["GETATTR"];
    assert_eq!(getattr.ops, 39919);
    assert_eq!(getattr.errors, 16);
    let read = &mount.operations["READ"];
    assert_eq!(read.ops, 175253);
    assert_eq!(read.bytes_recv, 183788522112);

    // The v3 per-op table has 22 operations (NULL through COMMIT).
    assert_eq!(mount.operations.len(), 22);

    // The mount uses nconnect=16: sixteen xprt lines, one per
    // connection, which must be aggregated mount-wide. Expected sums
    // were computed independently from the raw fixture with awk.
    let xprt = mount
        .xprt
        .as_ref()
        .expect("TCP xprt lines must be parsed on this mount");
    assert_eq!(xprt.nconnect, 16);
    assert_eq!(xprt.sends, 361460);
    assert_eq!(xprt.recvs, 361460);
    assert_eq!(xprt.bad_xids, 0);
    assert_eq!(xprt.req_u, 362021);
    assert_eq!(xprt.bklog_u, 0);
    assert_eq!(xprt.sending_u, 777);
    assert_eq!(xprt.pending_u, 173);
    assert_eq!(xprt.max_slots, 3, "max of per-connection HWMs");
}

#[test]
fn corpus_nfs41_impl_id_parses_fully() {
    // The impl_id: line used to abort the entire parse, making the
    // tool refuse to start against NFSv4.1+ servers that send an
    // implementation ID.
    let mounts = parse_fixture("nfs41_impl_id.txt");
    assert_eq!(mounts.len(), 1);

    let mount = &mounts["/mnt/data"];
    assert_eq!(mount.age, 86400);
    assert_eq!(mount.operations.len(), 5);
    assert_eq!(mount.operations["READ"].ops, 130);
    assert_eq!(mount.operations["WRITE"].ops, 256);
    assert_eq!(mount.operations["OPEN"].ops, 5);
    let xprt = mount.xprt.as_ref().expect("tcp xprt should parse");
    assert_eq!(xprt.protocol, "tcp");
}

#[test]
fn corpus_nfs3_fsc_parses_fully() {
    // The five-field fsc: line (fscache, kernels <= 5.17) used to
    // abort the entire parse. Note this fixture also uses 8-field
    // per-op lines (pre-5.3 kernels have no errors column).
    let mounts = parse_fixture("nfs3_fsc.txt");
    assert_eq!(mounts.len(), 1);

    let mount = &mounts["/mnt/cached"];
    assert_eq!(mount.operations.len(), 3);
    assert_eq!(mount.operations["READ"].ops, 256);
    assert_eq!(mount.operations["READ"].errors, 0);
    assert!(
        !mount.operations.contains_key("fsc"),
        "fsc is metadata, not an operation"
    );
}

#[test]
fn corpus_udp_xprt_is_absent_but_mount_parses() {
    // UDP transports have a different xprt layout the parser does not
    // (yet) map; the mount must still parse fully with xprt = None.
    let mounts = parse_fixture("nfs3_udp_xprt.txt");
    let mount = &mounts["/mnt/udp"];
    assert!(mount.xprt.is_none());
    assert_eq!(mount.operations["READ"].ops, 256);
}

#[test]
fn corpus_multi_mount_finds_only_nfs_mounts() {
    // Non-NFS device lines (rootfs, proc, ext4, tmpfs) interleaved
    // with two NFS mounts: exactly the two NFS mounts must surface,
    // each with its own stats — nothing bleeding across blocks.
    let mounts = parse_fixture("multi_mount.txt");
    assert_eq!(mounts.len(), 2, "exactly the two NFS mounts");

    let a = &mounts["/mnt/a"];
    assert_eq!(a.age, 1000);
    assert_eq!(a.operations.len(), 2);
    assert_eq!(a.operations["GETATTR"].ops, 500);
    assert!(!a.operations.contains_key("WRITE"));

    let b = &mounts["/mnt/b"];
    assert_eq!(b.age, 2000);
    assert_eq!(b.operations.len(), 2);
    assert_eq!(b.operations["WRITE"].ops, 300);
    assert!(!b.operations.contains_key("READ"));
}

#[test]
fn corpus_hostile_lines_lose_only_what_is_broken() {
    // The hostile fixture packs junk before any device line, unknown
    // metadata, a corrupt xprt line, broken per-op lines, and a
    // malformed device line. The parse must succeed, keeping every
    // well-formed mount and operation and dropping only the broken
    // bits.
    let mounts = parse_fixture("hostile_lines.txt");
    assert_eq!(mounts.len(), 2, "the two well-formed mounts survive");

    let good = &mounts["/mnt/good"];
    assert_eq!(good.age, 500);
    assert_eq!(good.operations.len(), 2, "READ and GETATTR survive");
    assert_eq!(good.operations["READ"].ops, 100);
    assert_eq!(good.operations["GETATTR"].ops, 500);
    assert!(!good.operations.contains_key("BROKEN"));
    assert!(!good.operations.contains_key("TRUNC"));
    // The valid tcp xprt line parses; the corrupted one is skipped
    // without clobbering it.
    let xprt = good.xprt.as_ref().expect("valid xprt line should stick");
    assert_eq!(xprt.sends, 1000);

    let alsogood = &mounts["/mnt/alsogood"];
    assert_eq!(alsogood.operations["WRITE"].ops, 300);
}

/// The core robustness property: no single-line corruption of any
/// realistic input may ever make the parser return an error. For every
/// line of every fixture we try three mutations — garbage without a
/// colon, a colon-line with non-numeric stats, and a truncation — and
/// require the parse to still succeed.
#[test]
fn corpus_mutations_never_abort_the_parse() {
    let fixtures = [
        "nfs3_nconnect16.txt",
        "nfs41_impl_id.txt",
        "nfs3_fsc.txt",
        "nfs3_udp_xprt.txt",
        "multi_mount.txt",
        "hostile_lines.txt",
    ];

    let mut parses = 0usize;
    for name in fixtures {
        let content = load_fixture(name);
        let lines: Vec<&str> = content.lines().collect();

        for i in 0..lines.len() {
            let mutations = [
                "x@@@!! junk without any colon".to_string(),
                "\tBOGUS: 1 2 three 4 5 6 7 8 9".to_string(),
                lines[i][..lines[i].len() / 2].to_string(),
            ];
            for mutation in mutations {
                let mut mutated: Vec<&str> = lines.clone();
                mutated[i] = &mutation;
                let text = mutated.join("\n");
                if let Err(e) = parse_mountstats_reader(Cursor::new(text)) {
                    panic!(
                        "mutating line {} of {name} to {:?} aborted the parse: {e}",
                        i + 1,
                        mutation
                    );
                }
                parses += 1;
            }
        }
    }
    assert!(parses > 300, "sanity: the mutation loop actually ran");
}

/// Whole-file abuse: reversed line order and appended binary-ish junk
/// must degrade gracefully (fewer mounts, missing fields) but never
/// error.
#[test]
fn corpus_gross_abuse_never_aborts_the_parse() {
    for name in ["nfs3_nconnect16.txt", "multi_mount.txt"] {
        let content = load_fixture(name);

        let reversed: String = content.lines().rev().collect::<Vec<_>>().join("\n");
        parse_mountstats_reader(Cursor::new(reversed))
            .unwrap_or_else(|e| panic!("reversed {name} aborted the parse: {e}"));

        let with_junk = format!("{content}\n\u{1}\u{2}garbage: \u{3}\nmore ??? junk\n");
        parse_mountstats_reader(Cursor::new(with_junk))
            .unwrap_or_else(|e| panic!("junk-suffixed {name} aborted the parse: {e}"));
    }
}
