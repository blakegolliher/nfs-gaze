//go:build linux

package internal

import (
	"flag"
	"fmt"
	"log"
	"os"
	"strconv"
	"strings"
	"time"
)

// InitFlags initializes and parses the command-line flags.
func InitFlags() (*string, *string, *time.Duration, *int, *bool, *bool, *bool, *string) {
	// Command-line flags
	var (
		mountPoint     = flag.String("m", "", "Mount point to monitor")
		operations     = flag.String("ops", "", "Comma-separated list of operations to monitor")
		interval       = flag.Duration("i", 1*time.Second, "Update interval")
		count          = flag.Int("c", 0, "Number of iterations (0 = infinite)")
		showAttr       = flag.Bool("attr", false, "Show attribute cache statistics")
		showBandwidth  = flag.Bool("bw", false, "Show bandwidth statistics")
		clearScreen    = flag.Bool("clear", false, "Clear screen between iterations")
		mountstatsPath = flag.String("f", "/proc/self/mountstats", "Path to mountstats file")
	)

	flag.Usage = func() {
		fmt.Fprintf(os.Stderr, "NFS I/O Statistics Monitor\n\n")
		fmt.Fprintf(os.Stderr, "Usage: %s [options] [mount_point] [interval] [count]\n\n", os.Args[0])
		fmt.Fprintf(os.Stderr, "Options:\n")
		flag.PrintDefaults()
		fmt.Fprintf(os.Stderr, "\nExamples:\n")
		fmt.Fprintf(os.Stderr, "  # Monitor specific operations with bandwidth\n")
		fmt.Fprintf(os.Stderr, "  %s -m /mnt/nfs -ops READ,WRITE -bw\n\n", os.Args[0])
		fmt.Fprintf(os.Stderr, "  # Clear screen between iterations\n")
		fmt.Fprintf(os.Stderr, "  %s -m /mnt/nfs --clear\n\n", os.Args[0])
	}

	// Parse positional arguments (for compatibility)
	nonFlagArgs := []string{}
	for i := 1; i < len(os.Args); i++ {
		if strings.HasPrefix(os.Args[i], "-") {
			break
		}
		nonFlagArgs = append(nonFlagArgs, os.Args[i])
	}

	if len(nonFlagArgs) > 0 && *mountPoint == "" {
		*mountPoint = nonFlagArgs[0]
	}
	if len(nonFlagArgs) > 1 {
		if intervalSec, err := strconv.Atoi(nonFlagArgs[1]); err == nil {
			*interval = time.Duration(intervalSec) * time.Second
		}
	}
	if len(nonFlagArgs) > 2 {
		if countVal, err := strconv.Atoi(nonFlagArgs[2]); err == nil {
			*count = countVal
		}
	}

	flag.Parse()
	return mountPoint, operations, interval, count, showAttr, showBandwidth, clearScreen, mountstatsPath
}

// parseOperationsFilter parses the comma-separated list of operations to monitor.
func ParseOperationsFilter(operations string) map[string]bool {
	var opsFilter map[string]bool
	if operations != "" {
		opsFilter = make(map[string]bool)
		for _, op := range strings.Split(operations, ",") {
			opsFilter[strings.TrimSpace(op)] = true
		}
	}
	return opsFilter
}

// getMountsToMonitor determines which mounts to monitor based on user input.
func GetMountsToMonitor(mountPoint string, previousMounts map[string]*NFSMount) []string {
	var monitorMounts []string
	if mountPoint != "" {
		if _, exists := previousMounts[mountPoint]; !exists {
			log.Fatalf("Mount point %s not found", mountPoint)
		}
		monitorMounts = append(monitorMounts, mountPoint)
	} else {
		for mp := range previousMounts {
			monitorMounts = append(monitorMounts, mp)
		}
		if len(monitorMounts) == 0 {
			fmt.Fprintf(os.Stderr, "No NFS mounts found\n")
			os.Exit(1)
		}
	}
	return monitorMounts
}

// printInitialSummary prints the initial summary of the monitored mounts.
func PrintInitialSummary(monitorMounts []string, previousMounts map[string]*NFSMount, opsFilter map[string]bool, showAttr bool, operations string, interval time.Duration) {
	// Print header for simple mode
	fmt.Printf("Monitoring NFS mount: %s (%s)\n", monitorMounts[0], previousMounts[monitorMounts[0]].Device)
	fmt.Printf("Update interval: %v\n", interval)
	if operations != "" {
		fmt.Printf("Filtering operations: %s\n", operations)
	}
}

// monitoringLoop is the main monitoring loop of the application.
func MonitoringLoop(sigChan chan os.Signal, interval time.Duration, count int, mountstatsPath string, clearScreen bool, monitorMounts []string, previousMounts map[string]*NFSMount, opsFilter map[string]bool, showAttr bool, showBandwidth bool, operations string) {
	// Monitoring loop
	iteration := 0
	ticker := time.NewTicker(interval)
	defer ticker.Stop()

	for {
		select {
		case <-ticker.C:
			iteration++

			// Read current stats
			currentMounts, err := ParseMountstats(mountstatsPath)
			if err != nil {
				log.Printf("Error reading mountstats: %v", err)
				continue
			}

			timestamp := time.Now()
			duration := interval.Seconds()

			// Clear screen if requested
			if clearScreen {
				fmt.Print("\033[H\033[2J")
				// Reprint header after clearing
				fmt.Printf("Monitoring NFS mount: %s (%s)\n", monitorMounts[0], currentMounts[monitorMounts[0]].Device)
				fmt.Printf("Update interval: %v | Time: %s\n", interval, timestamp.Format("15:04:05"))
				if operations != "" {
					fmt.Printf("Filtering operations: %s\n", operations)
				}
			}

			// Process each monitored mount
			for _, mp := range monitorMounts {
				currentMount, exists := currentMounts[mp]
				if !exists {
					continue
				}

				previousMount := previousMounts[mp]
				if previousMount == nil {
					continue
				}

				// Calculate deltas
				var stats []*DeltaStats

				for opName, currentOp := range currentMount.Operations {
					// Apply filter if specified
					if opsFilter != nil && !opsFilter[opName] {
						continue
					}

					previousOp := previousMount.Operations[opName]
					if previousOp != nil {
						delta := CalculateDelta(previousOp, currentOp, duration)
						if delta != nil && delta.DeltaOps > 0 {
							stats = append(stats, delta)
						}
					}
				}

				// Display results - only show if there's activity
				if len(stats) > 0 {
					DisplayStatsSimple(currentMount, stats, showBandwidth, timestamp)
				}
			}

			// Update previous stats
			previousMounts = currentMounts

			// Check iteration count
			if count > 0 && iteration >= count {
				return
			}

		case <-sigChan:
			fmt.Println("\nCaught ^C... exiting")
			return
		}
	}
}
