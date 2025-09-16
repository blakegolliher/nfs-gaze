use nfs_gaze::{parse_events, parse_nfs_operation, NFSEvents, NFSOperation};

#[test]
fn test_parse_events() {
    struct TestCase {
        name: &'static str,
        parts: Vec<&'static str>,
        want_err: bool,
        verify: Option<fn(&NFSEvents)>,
    }

    let tests = vec![
        TestCase {
            name: "valid events with all fields",
            parts: vec![
                "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15",
                "16", "17", "18", "19", "20", "21", "22", "23", "24", "25", "26", "27",
            ],
            want_err: false,
            verify: Some(|e: &NFSEvents| {
                assert_eq!(e.inode_revalidate, 1, "InodeRevalidate should be 1");
                assert_eq!(e.pnfs_write, 27, "PNFSWrite should be 27");
            }),
        },
        TestCase {
            name: "valid events without pNFS fields",
            parts: vec![
                "100", "200", "300", "400", "500", "600", "700", "800", "900", "1000", "1100",
                "1200", "1300", "1400", "1500", "1600", "1700", "1800", "1900", "2000", "2100",
                "2200", "2300", "2400", "2500",
            ],
            want_err: false, // This should now succeed with 25 parts
            verify: None,
        },
        TestCase {
            name: "insufficient parts",
            parts: vec!["1", "2", "3"],
            want_err: true,
            verify: None,
        },
        TestCase {
            name: "invalid number format",
            parts: vec![
                "a", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15",
                "16", "17", "18", "19", "20", "21", "22", "23", "24", "25", "26", "27",
            ],
            want_err: true,
            verify: None,
        },
    ];

    for test in tests {
        let parts: Vec<String> = test.parts.into_iter().map(|s| s.to_string()).collect();
        let result = parse_events(&parts);

        match (result, test.want_err) {
            (Ok(events), false) => {
                if let Some(verify) = test.verify {
                    verify(&events);
                }
            }
            (Err(_), true) => {
                // Expected error
            }
            (Ok(_), true) => {
                panic!("Test '{}': expected error but got success", test.name);
            }
            (Err(e), false) => {
                panic!("Test '{}': unexpected error: {}", test.name, e);
            }
        }
    }
}

#[test]
fn test_parse_nfs_operation() {
    struct TestCase {
        name: &'static str,
        op_name: &'static str,
        stats: Vec<&'static str>,
        want_err: bool,
        verify: Option<fn(&NFSOperation)>,
    }

    let tests = vec![
        TestCase {
            name: "valid operation with all fields",
            op_name: "READ",
            stats: vec!["100", "95", "5", "1024", "2048", "10", "20", "30", "2"],
            want_err: false,
            verify: Some(|op: &NFSOperation| {
                assert_eq!(op.name, "READ", "Name should be READ");
                assert_eq!(op.ops, 100, "Ops should be 100");
                assert_eq!(op.ntrans, 95, "Ntrans should be 95");
                assert_eq!(op.timeouts, 5, "Timeouts should be 5");
                assert_eq!(op.bytes_sent, 1024, "BytesSent should be 1024");
                assert_eq!(op.bytes_recv, 2048, "BytesRecv should be 2048");
                assert_eq!(op.queue_time, 10, "QueueTime should be 10");
                assert_eq!(op.rtt, 20, "RTT should be 20");
                assert_eq!(op.execute_time, 30, "ExecuteTime should be 30");
                assert_eq!(op.errors, 2, "Errors should be 2");
            }),
        },
        TestCase {
            name: "insufficient stats",
            op_name: "WRITE",
            stats: vec!["100", "95"],
            want_err: true,
            verify: None,
        },
        TestCase {
            name: "invalid number format",
            op_name: "READ",
            stats: vec!["abc", "95", "5", "1024", "2048", "10", "20", "30", "2"],
            want_err: true,
            verify: None,
        },
        TestCase {
            name: "zero values",
            op_name: "GETATTR",
            stats: vec!["0", "0", "0", "0", "0", "0", "0", "0", "0"],
            want_err: false,
            verify: Some(|op: &NFSOperation| {
                assert_eq!(op.name, "GETATTR", "Name should be GETATTR");
                assert_eq!(op.ops, 0, "Ops should be 0");
                assert_eq!(op.ntrans, 0, "Ntrans should be 0");
                assert_eq!(op.timeouts, 0, "Timeouts should be 0");
                assert_eq!(op.bytes_sent, 0, "BytesSent should be 0");
                assert_eq!(op.bytes_recv, 0, "BytesRecv should be 0");
                assert_eq!(op.queue_time, 0, "QueueTime should be 0");
                assert_eq!(op.rtt, 0, "RTT should be 0");
                assert_eq!(op.execute_time, 0, "ExecuteTime should be 0");
                assert_eq!(op.errors, 0, "Errors should be 0");
            }),
        },
    ];

    for test in tests {
        let stats: Vec<String> = test.stats.into_iter().map(|s| s.to_string()).collect();
        let result = parse_nfs_operation(test.op_name, &stats);

        match (result, test.want_err) {
            (Ok(operation), false) => {
                if let Some(verify) = test.verify {
                    verify(&operation);
                }
            }
            (Err(_), true) => {
                // Expected error
            }
            (Ok(_), true) => {
                panic!("Test '{}': expected error but got success", test.name);
            }
            (Err(e), false) => {
                panic!("Test '{}': unexpected error: {}", test.name, e);
            }
        }
    }
}
