//go:build linux

package main

import (
	"os"
	"os/signal"
	"syscall"
	"time"
	"log"
)

// main is the entry point of the application.
func main() {
	mountPoint, operations, interval, count, showAttr, showBandwidth, nfsiostatMode, clearScreen, mountstatsPath := initFlags()

	opsFilter := parseOperationsFilter(*operations)

	// Setup signal handling for graceful shutdown.
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, os.Interrupt, syscall.SIGTERM)

	// Perform an initial read of the mount stats.
	previousMounts, err := parseMountstats(*mountstatsPath)
	if err != nil {
		log.Fatal("Error reading mountstats: ", err)
	}

	// Determine which mounts to monitor based on user input.
	monitorMounts := getMountsToMonitor(*mountPoint, previousMounts)

	// Print the initial summary of the monitored mounts.
	printInitialSummary(*nfsiostatMode, monitorMounts, previousMounts, opsFilter, *showAttr, *operations, *interval)

	// Start the main monitoring loop.
	monitoringLoop(sigChan, *interval, *count, *mountstatsPath, *clearScreen, *nfsiostatMode, monitorMounts, previousMounts, opsFilter, *showAttr, *showBandwidth, *operations)
}
