fn main() -> anyhow::Result<()> {
    #[cfg(not(target_os = "linux"))]
    {
        eprintln!("This application only works on Linux");
        std::process::exit(1);
    }

    #[cfg(target_os = "linux")]
    run_linux()
}

#[cfg(target_os = "linux")]
fn run_linux() -> anyhow::Result<()> {
    use clap::Parser;
    use nfs_gaze::cli::{parse_operations_filter, Args};
    use nfs_gaze::monitor::Monitor;
    use nfs_gaze::parser::parse_mountstats;
    use std::io::stdout;
    use std::time::Duration;

    let args = Args::parse();

    // Parse operations filter
    let operations_filter = parse_operations_filter(args.operations);

    // Read initial mountstats to find available mounts
    let initial_mounts = match parse_mountstats(&args.mountstats_path) {
        Ok(mounts) => mounts,
        Err(e) => {
            eprintln!(
                "Error reading mountstats from {}: {}",
                args.mountstats_path, e
            );
            std::process::exit(1);
        }
    };

    if initial_mounts.is_empty() {
        eprintln!("No NFS mounts found in {}", args.mountstats_path);
        std::process::exit(1);
    }

    // Determine which mounts to monitor
    let monitor_mounts =
        match Monitor::get_mounts_to_monitor(args.mount_point.clone(), &initial_mounts) {
            Ok(mounts) => mounts,
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        };

    if monitor_mounts.is_empty() {
        eprintln!("No matching NFS mounts found to monitor");
        std::process::exit(1);
    }

    // Create monitor and setup signal handling
    let monitor = Monitor::new();
    monitor.setup_signal_handling();

    let mut stdout = stdout();

    // Print initial summary
    Monitor::print_initial_summary(
        &mut stdout,
        &args.mount_point,
        &monitor_mounts,
        &operations_filter,
    )?;

    // Convert interval from seconds to Duration
    let interval = Duration::from_secs(args.interval);

    // Start monitoring loop
    if let Err(e) = monitor.monitoring_loop(
        &mut stdout,
        &args.mountstats_path,
        monitor_mounts,
        operations_filter,
        interval,
        args.count,
        args.show_bandwidth,
        args.clear_screen,
    ) {
        eprintln!("Monitoring error: {}", e);
        std::process::exit(1);
    }

    println!("Monitoring stopped.");
    Ok(())
}
