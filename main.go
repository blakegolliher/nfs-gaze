package main

import (
	"fmt"
	"log"
	"os"
	"os/signal"
	"runtime"
	"syscall"

	"nfs-gazer/internal"
)

// main is the entry point of the application.
func main() {
	if runtime.GOOS != "linux" {
		fmt.Println("This tool is only supported on Linux.")
		os.Exit(1)
	}
	mountPoint, operations, interval, count, showAttr, showBandwidth, clearScreen, mountstatsPath := internal.InitFlags()

	opsFilter := internal.ParseOperationsFilter(*operations)

	// Setup signal handling for graceful shutdown.
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, os.Interrupt, syscall.SIGTERM)

	// Perform an initial read of the mount stats.
	previousMounts, err := internal.ParseMountstats(*mountstatsPath)
	if err != nil {
		log.Fatal("Error reading mountstats: ", err)
	}

	// Determine which mounts to monitor based on user input.
	monitorMounts := internal.GetMountsToMonitor(*mountPoint, previousMounts)

	// Print the initial summary of the monitored mounts.
	internal.PrintInitialSummary(monitorMounts, previousMounts, opsFilter, *showAttr, *operations, *interval)

	// Start the main monitoring loop.
	internal.MonitoringLoop(sigChan, *interval, *count, *mountstatsPath, *clearScreen, monitorMounts, previousMounts, opsFilter, *showAttr, *showBandwidth, *operations)
}
