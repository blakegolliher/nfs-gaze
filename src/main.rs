#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[cfg(not(target_os = "linux"))]
    {
        return Err(anyhow::anyhow!("This application only works on Linux"));
    }

    #[cfg(target_os = "linux")]
    run_linux().await
}

#[cfg(target_os = "linux")]
async fn run_linux() -> anyhow::Result<()> {
    use clap::Parser;
    use nfs_gaze::cli::{parse_operations_filter, Args};
    use nfs_gaze::metrics::MetricsManager;
    use nfs_gaze::monitor::{Monitor, MonitorConfig};
    use nfs_gaze::parser::parse_mountstats;
    use std::io::stdout;
    use std::time::Duration;

    let args = Args::parse();

    // Parse operations filter
    let operations_filter = parse_operations_filter(args.operations.clone());

    // Initialize metrics manager if observability features are enabled
    let metrics_config = args.to_metrics_config();
    let metrics_manager = if metrics_config.enable_prometheus {
        let manager = MetricsManager::new(metrics_config)
            .map_err(|e| anyhow::anyhow!("Failed to initialize metrics: {}", e))?;

        if manager.is_enabled() {
            // Start Prometheus HTTP server if enabled
            #[cfg(feature = "prometheus")]
            if args.prometheus {
                manager.start_prometheus_server();
                println!(
                    "Prometheus metrics available at http://{}:{}/metrics",
                    args.prometheus_bind, args.prometheus_port
                );
            }
        }
        Some(manager)
    } else {
        None
    };

    // Read initial mountstats to find available mounts
    let initial_mounts = parse_mountstats(&args.mountstats_path).map_err(|e| {
        anyhow::anyhow!(
            "Error reading mountstats from {}: {}",
            args.mountstats_path,
            e
        )
    })?;

    if initial_mounts.is_empty() {
        return Err(anyhow::anyhow!(
            "No NFS mounts found in {}",
            args.mountstats_path
        ));
    }

    // Determine which mounts to monitor
    let monitor_mounts = Monitor::get_mounts_to_monitor(args.mount_point.clone(), &initial_mounts)
        .map_err(|e| anyhow::anyhow!("Error: {}", e))?;

    if monitor_mounts.is_empty() {
        return Err(anyhow::anyhow!("No matching NFS mounts found to monitor"));
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
    let duration = args.duration.map(Duration::from_secs);

    // Start monitoring loop
    monitor
        .monitoring_loop(
            &mut stdout,
            MonitorConfig {
                mountstats_path: &args.mountstats_path,
                monitor_mounts,
                operations_filter,
                interval,
                count: args.count,
                duration,
                output: args.output.clone(),
                show_bandwidth: args.show_bandwidth,
                clear_screen: args.clear_screen,
                metrics_manager: metrics_manager.as_ref(),
            },
        )
        .map_err(|e| anyhow::anyhow!("Monitoring error: {}", e))?;

    println!("Monitoring stopped.");
    Ok(())
}
