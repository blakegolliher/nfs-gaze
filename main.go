//go:build linux

package main

import (
	"log"
	"os"
	"os/signal"
	"syscall"
)

// main is the entry point of the application.
func main() {
	flags := InitFlags()

	opsFilter := ParseOperationsFilter(flags.Operations)

	// Setup signal handling for graceful shutdown.
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, os.Interrupt, syscall.SIGTERM)

	// Perform an initial read of the mount stats.
	previousMounts, err := ParseMountstats(flags.MountstatsPath)
	if err != nil {
		log.Fatal("Error reading mountstats: ", err)
	}

	// Determine which mounts to monitor based on user input.
	monitorMounts, err := GetMountsToMonitor(flags.MountPoint, previousMounts)
	if err != nil {
		log.Fatal(err)
	}

	// Print the initial summary of the monitored mounts.
	PrintInitialSummary(flags, monitorMounts, previousMounts, opsFilter)

	// Start the main monitoring loop.
	MonitoringLoop(sigChan, flags, monitorMounts, previousMounts, opsFilter)
}
